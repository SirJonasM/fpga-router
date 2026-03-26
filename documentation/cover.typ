#let title-page(title:[], email:str, author: [], body) = {
  set page(margin: (top: 1.5in, rest: 2in))
  set text(size: 16pt)
  set heading(numbering: "1.1.1")
  line(start: (0%, 0%), end: (8.5in, 0%), stroke: (thickness: 2pt))
  align(horizon + left)[
    #text(size: 24pt, title)\
    #v(1em)
    FPGA Router written in Rust with STA
    #v(1em)
    #text(size:16pt, author)
    #linebreak()
    #v(1em)
    #linebreak()
    #link("mailto:" + email, email)
  ]
  align(bottom + left)[#datetime.today().display()]
  set page(fill: none, margin: auto)
  pagebreak()
  body
}

#show: body => title-page(
  title: [FPGA Router],
  email:"Jonas.Moewes@uni-heidelberg.de",
  author: [Jonas Möwes],
  body
)
