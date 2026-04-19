[
  "fn"
  "let"
  "mut"
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
  "?"
] @punctuation.delimiter

(iden) @variable
(func_name) @function
(param_decl) @variable.parameter 


(str_lit) @string
(str_lit (escape_seq) @string.escape) 

(int_lit) @number

[
  "true"
  "false"
  "null"
] @constant.builtin

(comment) @comment
