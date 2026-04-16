[
  "fn"
  "return"
  "break"
  "continue"
  "if"
  "else"
  "while"
  "loop"
] @keyword

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

[
  "-"
  "-="
  "+"
  "+="
  "*"
  "*="
  "/"
  "/="
  "%"
  "%="
  "<"
  "<="
  "="
  "=="
  "!"
  "!="
  ">"
  ">="
  "&&"
  "||"
] @operator

[
  ";"
  ":"
  ","
] @punctuation.delimiter

(iden) @variable
(func_name (iden) @function)
(func_param (iden) @variable.parameter)


(str_lit) @string
(int_lit) @number

[
  "true"
  "false"
  "null"
] @constant.builtin

(comment) @comment
