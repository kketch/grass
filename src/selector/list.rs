use std::collections::VecDeque;
use std::{
    fmt::{self, Write},
    mem,
};

use super::{unify_complex, ComplexSelector, ComplexSelectorComponent};

use crate::common::{Brackets, ListSeparator, QuoteKind};
use crate::value::Value;

/// A selector list.
///
/// A selector list is composed of `ComplexSelector`s. It matches an element
/// that matches any of the component selectors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectorList {
    /// The components of this selector.
    ///
    /// This is never empty.
    pub components: Vec<ComplexSelector>,
}

impl fmt::Display for SelectorList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let complexes = self.components.iter().filter(|c| !c.is_invisible());

        let mut first = true;

        for complex in complexes {
            if first {
                first = false;
            } else {
                f.write_char(',')?;
                if complex.line_break {
                    f.write_char('\n')?;
                } else {
                    // todo: not emitted in compressed
                    f.write_char(' ')?;
                }
            }
            write!(f, "{}", complex)?;
        }
        Ok(())
    }
}

impl SelectorList {
    pub fn is_invisible(&self) -> bool {
        self.components.iter().all(|c| c.is_invisible())
    }

    pub fn contains_parent_selector(&self) -> bool {
        self.components.iter().any(|c| c.contains_parent_selector())
    }

    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Returns a SassScript list that represents this selector.
    ///
    /// This has the same format as a list returned by `selector-parse()`.
    pub fn to_sass_list(self) -> Value {
        Value::List(
            self.components
                .into_iter()
                .map(|complex| {
                    Value::List(
                        complex
                            .components
                            .into_iter()
                            .map(|complex_component| {
                                Value::String(complex_component.to_string(), QuoteKind::None)
                            })
                            .collect(),
                        ListSeparator::Space,
                        Brackets::None,
                    )
                })
                .collect(),
            ListSeparator::Comma,
            Brackets::None,
        )
    }

    /// Returns a `SelectorList` that matches only elements that are matched by
    /// both this and `other`.
    ///
    /// If no such list can be produced, returns `None`.
    pub fn unify(self, other: Self) -> Option<Self> {
        let contents: Vec<ComplexSelector> = self
            .components
            .into_iter()
            .flat_map(|c1| {
                other.clone().components.into_iter().flat_map(move |c2| {
                    let unified: Option<Vec<Vec<ComplexSelectorComponent>>> =
                        unify_complex(vec![c1.components.clone(), c2.components]);
                    if let Some(u) = unified {
                        u.into_iter()
                            .map(|c| ComplexSelector {
                                components: c,
                                line_break: false,
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                })
            })
            .collect();

        if contents.is_empty() {
            return None;
        }

        Some(Self {
            components: contents,
        })
    }

    /// Returns a new list with all `SimpleSelector::Parent`s replaced with `parent`.
    ///
    /// If `implicit_parent` is true, this treats `ComplexSelector`s that don't
    /// contain an explicit `SimpleSelector::Parent` as though they began with one.
    ///
    /// The given `parent` may be `None`, indicating that this has no parents. If
    /// so, this list is returned as-is if it doesn't contain any explicit
    /// `SimpleSelector::Parent`s. If it does, this returns a `SassError`.
    // todo: return SassResult<Self> (the issue is figuring out the span)
    pub fn resolve_parent_selectors(self, parent: Option<Self>, implicit_parent: bool) -> Self {
        let parent = match parent {
            Some(p) => p,
            None => {
                if !self.contains_parent_selector() {
                    return self;
                }
                todo!("Top-level selectors may not contain the parent selector \"&\".")
            }
        };

        Self {
            components: flatten_vertically(
                self.components
                    .into_iter()
                    .map(|complex| {
                        if !complex.contains_parent_selector() {
                            if !implicit_parent {
                                return vec![complex];
                            }
                            return parent
                                .clone()
                                .components
                                .into_iter()
                                .map(move |parent_complex| {
                                    let mut components = parent_complex.components;
                                    components.append(&mut complex.components.clone());
                                    ComplexSelector {
                                        components,
                                        line_break: complex.line_break || parent_complex.line_break,
                                    }
                                })
                                .collect();
                        }

                        let mut new_complexes: Vec<Vec<ComplexSelectorComponent>> =
                            vec![Vec::new()];
                        let mut line_breaks = vec![false];

                        for component in complex.components {
                            if component.is_compound() {
                                let resolved = match component
                                    .clone()
                                    .resolve_parent_selectors(parent.clone())
                                {
                                    Some(r) => r,
                                    None => {
                                        for new_complex in new_complexes.iter_mut() {
                                            new_complex.push(component.clone());
                                        }
                                        continue;
                                    }
                                };

                                let previous_complexes = mem::take(&mut new_complexes);
                                let previous_line_breaks = mem::take(&mut line_breaks);

                                let mut i = 0;
                                for new_complex in previous_complexes {
                                    let line_break = previous_line_breaks[i];
                                    i += 1;
                                    for mut resolved_complex in resolved.clone() {
                                        let mut new_this_complex = new_complex.clone();
                                        new_this_complex.append(&mut resolved_complex.components);
                                        new_complexes.push(mem::take(&mut new_this_complex));
                                        line_breaks.push(line_break || resolved_complex.line_break);
                                    }
                                }
                            } else {
                                for new_complex in new_complexes.iter_mut() {
                                    new_complex.push(component.clone());
                                }
                            }
                        }

                        let mut i = 0;
                        return new_complexes
                            .into_iter()
                            .map(|new_complex| {
                                i += 1;
                                ComplexSelector {
                                    components: new_complex,
                                    line_break: line_breaks[i - 1],
                                }
                            })
                            .collect();
                    })
                    .collect(),
            ),
        }
    }

    pub fn is_superselector(&self, other: &Self) -> bool {
        other.components.iter().all(|complex1| {
            self.components
                .iter()
                .any(|complex2| complex2.is_super_selector(complex1))
        })
    }
}

fn flatten_vertically<A: std::fmt::Debug>(iterable: Vec<Vec<A>>) -> Vec<A> {
    let mut queues: Vec<VecDeque<A>> = iterable
        .into_iter()
        .map(|inner| VecDeque::from(inner))
        .collect();

    let mut result = Vec::new();

    while !queues.is_empty() {
        for queue in queues.iter_mut() {
            if queue.is_empty() {
                continue;
            }
            result.push(queue.pop_front().unwrap());
        }

        queues = queues
            .into_iter()
            .filter(|queue| !queue.is_empty())
            .collect();
    }

    result
}
