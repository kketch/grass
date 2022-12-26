use grass::InputSyntax;

#[macro_use]
mod macros;

test!(
    function_call,
    "a {
        color: rotate(-45deg);
    }",
    "a {\n  color: rotate(-45deg);\n}\n",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    retains_null,
    "a {
        color: null;
    }",
    "a {\n  color: null;\n}\n",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    does_not_evaluate_and,
    "a {
        color: 1 and 2;
    }",
    "a {\n  color: 1 and 2;\n}\n",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    does_not_evaluate_or,
    "a {
        color: 1 or 2;
    }",
    "a {\n  color: 1 or 2;\n}\n",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    does_not_evaluate_not,
    "a {
        color: not 2;
        color: not true;
        color: not false;
    }",
    "a {\n  color: not 2;\n  color: not true;\n  color: not false;\n}\n",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    denies_silent_comment,
    "// silent",
    "Error: Silent comments aren't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    denies_function_rule,
    "@function foo() {
        @return 2;
    }",
    "Error: This at-rule isn't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    denies_content_rule,
    "@content",
    "Error: This at-rule isn't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    allows_media_rule,
    "@media (foo) {
        a {
            color: red;
        }
    }",
    "@media (foo) {\n  a {\n    color: red;\n  }\n}\n",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    allows_var_empty_second_arg,
    "a {
        color: var(1, );
    }",
    "a {\n  color: var(1, );\n}\n",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_empty_second_arg_in_non_var_function,
    "a {
        color: foo(1, );
    }",
    "Error: Expected expression.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_if_function,
    "a {
        color: if(true, a, b);
    }",
    "Error: This function isn't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_map_get_function,
    "a {
        color: map-get(true, a, b);
    }",
    "Error: This function isn't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_plus_operator,
    "a {
        color: 1 + 2;
    }",
    "Error: Operators aren't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_parens,
    "a {
        color: (a b);
    }",
    "Error: Parentheses aren't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_variable_expr,
    "a {
        color: $a;
    }",
    "Error: Sass variables aren't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_parent_selector_expr,
    "a {
        color: &;
    }",
    "Error: The parent selector isn't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_unary_plus,
    "a {
        color: +(1);
    }",
    "Error: Operators aren't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_unary_minus,
    "a {
        color: -(1);
    }",
    "Error: Operators aren't allowed in plain CSS.",
    grass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    allows_rgb_function,
    "a {
        color: rgb(true, a, b);
    }",
    "a {\n  color: rgb(true, a, b);\n}\n",
    grass::Options::default().input_syntax(InputSyntax::Css)
);