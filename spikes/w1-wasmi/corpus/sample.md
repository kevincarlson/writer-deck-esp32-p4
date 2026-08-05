# The Salt Road

A draft chapter. This corpus deliberately contains the cases R-04-02 names as
the ones that break line-independent classification.

## Setext heading follows
This is a setext H2
-------------------

Ordinary paragraph text with *emphasis*, **strong**, `inline code`, and a
[reference link][salt] that resolves at the bottom of the file.

- list item one
- list item two
  - nested item at depth two
  - nested item with `code`
    - depth three
- list item three

1. ordered item
2. ordered item with a very long line that wraps in the editor viewport and
   continues onto a second physical line without a hard break
3. ordered item

> A blockquote line.
> A second blockquote line.
>
> > Nested blockquote.

```rust
// A fenced block. Everything in here is code, including lines that look
// like Markdown:
# not a heading
- not a list item
> not a quote
fn main() {
    let x = 42;
}
```

Text after the fence closes. This line must classify as a paragraph, which is
only correct if the fence state was tracked.

~~~
A tilde fence, which must also be tracked, and must not be closed by the
backtick form above.
~~~

| column a | column b |
|---|---|
| cell | cell |
| cell | cell |

Another Setext H1
=================

Trailing paragraph with a footnote-ish marker and an ![icon][ref] that is not
an image node in 1.0.

    an indented code block
    second line of it

Final paragraph.

[salt]: https://example.invalid/salt
[ref]: https://example.invalid/ref
