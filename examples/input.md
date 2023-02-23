# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

## Horizontal Rules

___

---

***

## Emphasis

**This is bold text**

__This is bold text__

*This is italic text*

_This is italic text_

~~Strikethrough~~

## Links

[link text][1]

[link with title][2]

This is [an example](http://example.com/ "Title") inline link.

[This link](http://example.net/) has no title attribute.

## Blockquotes

> Blockquotes can also be nested...
>> ...by using additional greater-than signs right next to each other...
> > > ...or with spaces between arrows.

## Indentation

  indentation 1-1

indentation 1-2
    indentation 2-1
    

## Code

Inline `code`

Indented code

    // Some comments
    line 1 of code
    line 2 of code
    line 3 of code


Block code "fences"

```
Sample text here...
```

Syntax highlighting

```rust
pub fn main() {
    println!("Hello world!");
}
```

## Unordered lists

- Sub-lists are made by indenting 2 spaces:
  - Children are also nested

    ```c
    printf("Hello world");
    ```

## Tables

| Option | Description |
| ------ | ----------- |
| data   | path to data files to supply the data that will be passed into templates. |
| engine | engine to be used for processing templates. Handlebars is the default. |

## Images

![Rust logo](./examples/img/rust.png)



[1]: http://example.com
[2]: http://example.com/ "title text!"
[3]: http://example.com
