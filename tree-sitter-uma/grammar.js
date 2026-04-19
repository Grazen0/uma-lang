/**
 * @file Uma grammar for tree-sitter
 * @author Grazen
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "uma",
  rules: {
    source_file: ($) => repeat($.func),
    func: ($) =>
      seq(
        "fn",
        alias($.iden, $.func_name),
        "(",
        optional(
          seq($.param_decl, repeat(seq(",", $.param_decl)), optional(",")),
        ),
        ")",
        "{",
        repeat($.stmt),
        "}",
      ),
    param_decl: ($) => seq(optional("mut"), $.iden),
    stmt: ($) =>
      choice(
        prec.left(
          0,
          seq("if", "(", $.expr, ")", $.stmt, optional(seq("else", $.stmt))),
        ),
        seq(
          "while",
          "(",
          $.expr,
          ")",
          optional(seq(":", "(", $.expr, ")")),
          $.stmt,
        ),
        seq("loop", $.stmt),
        seq("return", optional($.expr), ";"),
        seq("break", ";"),
        seq("continue", ";"),
        seq($.expr, ";"),
        $.decl_stmt,
        $.stmt_blk,
      ),

    decl_stmt: ($) =>
      seq("let", optional("mut"), field("left", $.iden), "=", $.expr, ";"),
    stmt_blk: ($) => prec(1, seq("{", repeat($.stmt), "}")),

    expr: ($) => $.assign_expr,
    assign_expr: ($) =>
      choice(
        $.ter_expr,
        seq(
          $.ter_expr,
          choice("=", "+=", "-=", "*=", "/=", "%="),
          $.assign_expr,
        ),
      ),
    ter_expr: ($) =>
      seq($.or_expr, optional(seq("?", $.expr, ":", $.ter_expr))),
    or_expr: ($) => seq(optional(seq($.or_expr, "||")), $.and_expr),
    and_expr: ($) => seq(optional(seq($.and_expr, "&&")), $.eq_expr),
    eq_expr: ($) =>
      seq(optional(seq($.eq_expr, choice("==", "!="))), $.ineq_expr),
    ineq_expr: ($) =>
      seq(optional(seq($.ineq_expr, choice("<", "<=", ">", ">="))), $.add_expr),
    add_expr: ($) =>
      seq(optional(seq($.add_expr, choice("+", "-"))), $.mul_expr),
    mul_expr: ($) =>
      seq(optional(seq($.mul_expr, choice("*", "/", "%"))), $.unary_expr),
    unary_expr: ($) => seq(optional(choice("+", "-", "!")), $.access_expr),
    access_expr: ($) => seq($.base_expr, repeat(seq("[", $.expr, "]"))),
    dict_entry: ($) => seq($.expr, ":", $.expr),
    base_expr: ($) =>
      choice(
        seq("(", $.expr, ")"),
        $.iden,
        seq(
          alias($.iden, $.func_name),
          "(",
          optional(seq($.expr, repeat(seq(",", $.expr)), optional(","))),
          ")",
        ),
        seq(
          "[",
          optional(seq($.expr, repeat(seq(",", $.expr)), optional(","))),
          "]",
        ),
        seq(
          "{",
          optional(
            seq($.dict_entry, repeat(seq(",", $.dict_entry)), optional(",")),
          ),
          "}",
        ),
        "null",
        "true",
        "false",
        $.int_lit,
        $.str_lit,
      ),

    int_lit: ($) => /\d+/,

    str_lit: ($) =>
      seq('"', repeat(choice($.unescaped_string_fragment, $.escape_seq)), '"'),
    unescaped_string_fragment: (_) => token.immediate(prec(1, /[^"\\\r\n]+/)),
    escape_seq: ($) => token.immediate(seq("\\", choice("\\", "n", "r", "0"))),

    iden: ($) => /[a-zA-Z_][a-zA-Z\d_]*/,

    comment: ($) => token(seq("#", /[^\r\n\u2028\u2029]*/)),
  },
  extras: ($) => [$.comment, /[\s\uFEFF\u2028\u2029\u2060\u200B]/],
});
