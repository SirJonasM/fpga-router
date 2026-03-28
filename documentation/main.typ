#set page(paper: "a4")
#include "cover.typ"
#counter(page).update(1)
#set page(numbering: "1.")
#set heading(numbering: "1.")
#set text(
  font: "New Computer Modern",
  size: 12pt,
)

#set page(header: text(size: 10pt)[#grid(
  columns: (1fr, 2fr, 1fr),
  column-gutter: 1fr,
  align: (left, center, right),
  [ACF], [University Heidelberg], [Jonas Möwes],
)])

= Overview
#lorem(100)
= Implementation
#lorem(100)
= Build Guide
#lorem(100)
= Tools and Data Used
#lorem(100)
= Further Notes
#lorem(100)

