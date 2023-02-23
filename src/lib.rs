//! A library to render different flavors of the Commonmark (markdown) language as GTK4 widgets.
//!
//! ## Example
//!
//! ```rust
//! use gtk4_commonmark::{render_input, RenderConfig};
//! use relm4::{adw, gtk, RelmApp, RelmContainerExt};
//! const INPUT_MARKDOWN: &str = include_str!("../examples/input.md");
//! gtk::init();
//!
//! // Parse the input and get a `gtk::Viewport` in return
//! let viewport = render_input(INPUT_MARKDOWN, RenderConfig::default()).unwrap();
//!
//! // Create a scrollable area to put the viewport in, with a clamp to limit its width
//! let scrollable = gtk::ScrolledWindow::new();
//! scrollable.container_add(&viewport);
//! let clamp = adw::Clamp::builder().maximum_size(800).build();
//! clamp.container_add(&scrollable);
//!
//! // Use the created widgets
//! // root.container_add(&clamp);
//! ```

use anyhow::Context;
use gtk::{
    gdk::Display,
    traits::{BoxExt, GridExt, WidgetExt},
    CssProvider, StyleContext,
};
use html2pango::html_escape;
pub use markdown::ParseOptions;
use markdown::{self, mdast::Node};
use syntect::{
    self,
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

#[derive(Debug, Clone)]
pub enum ImageSetting {
    /// Do not show images
    Ignore,
    /// Show images from their path on disk
    FromPath,
    /// Show images by embedding them at compilation time
    IncludeBytes,
}

/// Render configuration options.
///
/// Default implementation uses the "base16-mocha.dark" theme for code highlighting
/// and a parser for the Github flavored Markdown.
#[derive(Debug)]
pub struct RenderConfig<'a> {
    image_settings: ImageSetting,
    /// Configuration that describes how to parse markdown
    parse_options: ParseOptions,
    /// For available themes, please refer to the [syntect](https://github.com/trishume/syntect) documentation.
    highlight_theme: &'a str,
}

impl Default for RenderConfig<'_> {
    fn default() -> Self {
        Self {
            image_settings: ImageSetting::FromPath,
            parse_options: ParseOptions::gfm(),
            highlight_theme: "base16-mocha.dark",
        }
    }
}

/// Create widgets from commonmark input and return them in a new `gtk::Viewport`.
///
/// ## Errors
///
/// The only errors that can occur are from the commonmark parser crate [markdown-rs](https://github.com/wooorm/markdown-rs),
/// which states that only the MDX commonmark extension can have syntax errors.
///
/// ## Logging
///
/// Warning logs will be emitted if:
///
/// - a code block language name is invalid
/// - the provided syntect theme name is invalid
///
/// For available themes, please refer to the [syntect](https://github.com/trishume/syntect) documentation.
pub fn render_input(input: &str, render_config: RenderConfig) -> anyhow::Result<gtk::Viewport> {
    // Init synctect
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntect_ctx = SyntectCtx {
        ps: &ps,
        ts: &ts,
        theme_name: render_config.highlight_theme,
    };
    load_css();

    // Init viewport and content box
    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .name("common_mark_content_box")
        .width_request(100)
        .spacing(10)
        .margin_bottom(10)
        .margin_top(10)
        .margin_start(10)
        .margin_end(10)
        .valign(gtk::Align::Center)
        .build();
    let viewport = gtk::Viewport::builder()
        .name("commonmark_viewport")
        .vscroll_policy(gtk::ScrollablePolicy::Natural)
        .build();
    viewport.set_child(Some(&content_box));

    // Read commonmark
    let tree = markdown::to_mdast(input, &render_config.parse_options)
        .map_err(anyhow::Error::msg)
        .with_context(|| "commonmark parsing error")?;
    if let Some(children) = tree.children() {
        append_widgets_from_children(
            children,
            &content_box,
            None,
            &syntect_ctx,
            &mut 0,
            None,
            &render_config,
        );
    }

    Ok(viewport)
}

/// Append a string to a label
fn label_append(label: &gtk::Label, text: &str) {
    label.set_label(&format!("{}{}", label.label(), text));
}

struct SyntectCtx<'a> {
    ps: &'a SyntaxSet,
    ts: &'a ThemeSet,
    theme_name: &'a str,
}

#[derive(Clone)]
struct TableContext<'a> {
    table_grid: &'a gtk::Grid,
    current_row: i32,
    current_column: i32,
}

/// Append widgets to the root `gtk::Box`.
fn append_widgets_from_children<'a>(
    children: &[Node],
    root: &gtk::Box,
    current_label: Option<&gtk::Label>,
    syntect_ctx: &SyntectCtx<'a>,
    list_indent_level: &mut u16,
    table_ctx: Option<TableContext>,
    render_config: &RenderConfig,
) {
    let mut created_labels: Vec<gtk::Label> = Vec::new();
    let mut table_ctx = table_ctx;

    for child in children {
        match child {
            Node::Heading(heading) => {
                // A heading is a box with a label and a horizontal separator
                let heading_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(5)
                    .name("commonmark_heading_box")
                    .build();
                let (size_text, sep_height) = match heading.depth {
                    1 => ("xx-large", 4),
                    2 => ("x-large", 3),
                    3 => ("large", 2),
                    4 => ("medium", 1),
                    5 => ("medium", 1),
                    6 => ("medium", 1),
                    _ => continue,
                };

                let label = gtk::Label::builder()
                    .label(format!("<span font_size=\"{size_text}\">",))
                    .justify(gtk::Justification::Left)
                    .halign(gtk::Align::Start)
                    .build();

                let separator = gtk::Separator::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .height_request(sep_height)
                    .build();
                heading_box.append(&label);
                heading_box.append(&separator);
                root.append(&heading_box);
                append_widgets_from_children(
                    &heading.children,
                    &heading_box,
                    Some(&label),
                    syntect_ctx,
                    list_indent_level,
                    None,
                    render_config,
                );
                label_append(&label, "</span>");
                created_labels.push(label);
            }
            Node::Text(text) => {
                if let Some(label) = current_label {
                    label.set_label(&format!("{}{}", label.label(), html_escape(&text.value)));
                }
            }
            Node::Paragraph(node) => {
                if let Some(label) = current_label {
                    append_widgets_from_children(
                        &node.children,
                        root,
                        Some(label),
                        syntect_ctx,
                        list_indent_level,
                        None,
                        render_config,
                    );
                } else {
                    let paragraph_box = gtk::Box::builder()
                        .name("commonmark_paragraph_box")
                        .orientation(gtk::Orientation::Horizontal)
                        .vexpand(true)
                        .build();
                    let paragraph_label = empty_gtk_label();
                    paragraph_box.append(&paragraph_label);
                    append_widgets_from_children(
                        &node.children,
                        &paragraph_box,
                        Some(&paragraph_label),
                        syntect_ctx,
                        list_indent_level,
                        None,
                        render_config,
                    );
                    root.append(&paragraph_box);
                    created_labels.push(paragraph_label);
                }
            }
            Node::BlockQuote(block_quote) => {
                let block_quote_outer_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .spacing(15)
                    .name("commonmark_block_quote_outer_box")
                    .build();
                block_quote_outer_box.set_opacity(0.7);
                block_quote_outer_box.append(
                    &gtk::Separator::builder()
                        .orientation(gtk::Orientation::Vertical)
                        .width_request(5)
                        .build(),
                );
                let block_quote_inner_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(5)
                    .name("commonmark_block_quote_inner_box")
                    .build();
                append_widgets_from_children(
                    &block_quote.children,
                    &block_quote_inner_box,
                    None,
                    syntect_ctx,
                    list_indent_level,
                    None,
                    render_config,
                );
                block_quote_outer_box.append(&block_quote_inner_box);
                root.append(&block_quote_outer_box);
            }
            Node::Strong(node) => {
                if let Some(label) = current_label {
                    label_append(label, "<b>");
                    append_widgets_from_children(
                        &node.children,
                        root,
                        Some(label),
                        syntect_ctx,
                        list_indent_level,
                        None,
                        render_config,
                    );
                    label_append(label, "</b>");
                }
            }
            Node::Emphasis(node) => {
                if let Some(label) = current_label {
                    label_append(label, "<i>");
                    append_widgets_from_children(
                        &node.children,
                        root,
                        Some(label),
                        syntect_ctx,
                        list_indent_level,
                        None,
                        render_config,
                    );
                    label_append(label, "</i>");
                }
            }
            Node::Break(_) => {
                if let Some(label) = current_label {
                    label_append(label, "\n");
                }
            }
            Node::List(list) => {
                let list_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .name("commonmark_list_box")
                    .build();
                root.append(&list_box);
                *list_indent_level += 1;
                append_widgets_from_children(
                    &list.children,
                    &list_box,
                    None,
                    syntect_ctx,
                    list_indent_level,
                    None,
                    render_config,
                );
                *list_indent_level -= 1;
            }
            Node::ListItem(item) => {
                let item_outer_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .margin_start(((*list_indent_level as i32) - 1) * 15)
                    .valign(gtk::Align::Start)
                    .build();
                item_outer_box.append(
                    &gtk::Label::builder()
                        .label("- ")
                        .margin_top(3) // to align with checkbox
                        .valign(gtk::Align::Start)
                        .build(),
                );
                if let Some(checked) = item.checked {
                    let checked = gtk::CheckButton::builder()
                        .active(checked)
                        .valign(gtk::Align::Start)
                        .can_focus(false)
                        .focusable(false)
                        .build();
                    item_outer_box.append(&checked);
                }
                root.append(&item_outer_box);
                let item_inner_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .margin_top(3) // to align with checkbox
                    .name("commonmark_list_item_box")
                    .build();
                item_outer_box.append(&item_inner_box);
                append_widgets_from_children(
                    &item.children,
                    &item_inner_box,
                    None,
                    syntect_ctx,
                    list_indent_level,
                    None,
                    render_config,
                );
            }
            Node::InlineCode(inline_code) => {
                if let Some(label) = current_label {
                    label_append(
                        label,
                        &format!(
                            " <span><tt>{}</tt></span> ",
                            html_escape(&inline_code.value)
                        ),
                    );
                }
            }
            Node::Delete(node) => {
                if let Some(label) = current_label {
                    label_append(label, "<s>");
                    append_widgets_from_children(
                        &node.children,
                        root,
                        Some(label),
                        syntect_ctx,
                        list_indent_level,
                        None,
                        render_config,
                    );
                    label_append(label, "</s>");
                }
            }
            Node::Code(code_node) => {
                parse_code_block(
                    code_node.lang.as_ref(),
                    syntect_ctx.ps,
                    syntect_ctx.ts,
                    syntect_ctx.theme_name,
                    &code_node.value,
                    root,
                );
            }
            Node::Link(link) => {
                if let Some(link_label) = current_label {
                    label_append(link_label, &format!("<u><a href=\"{}\">", link.url));
                    if let Some(title) = &link.title {
                        label_append(link_label, &format!("{}</a></u>", title));
                    } else {
                        append_widgets_from_children(
                            &link.children,
                            root,
                            Some(link_label),
                            syntect_ctx,
                            list_indent_level,
                            None,
                            render_config,
                        );
                        label_append(link_label, "</a></u>");
                    }
                }
            }
            Node::Table(table) => {
                let table_grid = gtk::Grid::new();
                append_widgets_from_children(
                    &table.children,
                    root,
                    None,
                    syntect_ctx,
                    list_indent_level,
                    Some(TableContext {
                        table_grid: &table_grid,
                        current_row: 0,
                        current_column: 0,
                    }),
                    render_config,
                );

                root.append(&table_grid);
            }
            Node::TableRow(table_row) => {
                if let Some(mut ctx) = table_ctx {
                    ctx.current_row += 1;
                    ctx.current_column = 0;
                    append_widgets_from_children(
                        &table_row.children,
                        root,
                        None,
                        syntect_ctx,
                        list_indent_level,
                        Some(ctx.clone()),
                        render_config,
                    );
                    table_ctx = Some(ctx);
                }
            }
            Node::TableCell(table_cell) => {
                if let Some(mut ctx) = table_ctx {
                    let cell_label = empty_gtk_label();
                    cell_label.set_margin_bottom(4);
                    cell_label.set_margin_end(4);
                    cell_label.set_margin_start(4);
                    cell_label.set_margin_top(4);
                    cell_label.set_hexpand(true);
                    cell_label.set_vexpand(true);
                    let cell_outer_box = gtk::Box::builder()
                        .orientation(gtk::Orientation::Vertical)
                        .spacing(0)
                        .css_classes(vec!["table_outer_box"])
                        .build();
                    let cell_inner_box = gtk::Box::builder()
                        .orientation(gtk::Orientation::Vertical)
                        .css_classes(vec!["table_inner_box"])
                        .margin_bottom(1)
                        .margin_end(1)
                        .margin_start(1)
                        .margin_top(1)
                        .spacing(5)
                        .build();
                    cell_inner_box.append(&cell_label);
                    cell_outer_box.append(&cell_inner_box);
                    ctx.table_grid.attach(
                        &cell_outer_box,
                        ctx.current_column,
                        ctx.current_row - 1,
                        1,
                        1,
                    );
                    ctx.current_column += 1;

                    append_widgets_from_children(
                        &table_cell.children,
                        &cell_inner_box,
                        Some(&cell_label),
                        syntect_ctx,
                        list_indent_level,
                        Some(ctx.clone()),
                        render_config,
                    );
                    table_ctx = Some(ctx);
                    created_labels.push(cell_label);
                }
            }
            Node::ThematicBreak(_) => {
                let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
                root.append(&sep);
            }
            Node::Image(image) => match render_config.image_settings {
                ImageSetting::Ignore => continue,
                ImageSetting::FromPath => {
                    let picture = gtk::Picture::for_filename(&image.url);
                    picture.set_hexpand(true);
                    picture.set_vexpand(true);
                    picture.set_can_shrink(true);
                    picture.set_content_fit(gtk::ContentFit::Contain);
                    root.append(&picture);
                }
                ImageSetting::IncludeBytes => todo!(),
            },
            // Nodes below are not currently supported
            Node::FootnoteReference(_) => {}
            Node::LinkReference(_) => {}
            Node::Definition(_) => {}
            Node::ImageReference(_) => {}
            Node::Math(_) => {}
            Node::InlineMath(_) => {}
            Node::Html(_) => {}
            Node::MdxjsEsm(_) => {}
            Node::Toml(_) => {}
            Node::Yaml(_) => {}
            Node::FootnoteDefinition(_) => {}
            Node::MdxJsxFlowElement(_) => {}
            Node::MdxJsxTextElement(_) => {}
            Node::MdxTextExpression(_) => {}
            Node::MdxFlowExpression(_) => {}
            Node::Root(_) => {}
        }
    }

    for created_label in &created_labels {
        created_label.set_use_markup(true);
    }
}

fn empty_gtk_label() -> gtk::Label {
    gtk::Label::builder()
        .justify(gtk::Justification::Left)
        .halign(gtk::Align::Start)
        .use_markup(false) // set it to `true` at then end to avoid GTK warnings
        .wrap(true)
        .label("")
        .build()
}

/// Converts a code block to widgets that are appended to the root `gtk::Box`. Code is syntax highlighted.
fn parse_code_block(
    language_name: Option<&String>,
    ps: &SyntaxSet,
    ts: &ThemeSet,
    highlight_theme_name: &str,
    content: &str,
    root: &gtk::Box,
) {
    let outer_box = gtk::Box::builder()
        .css_classes(vec!["code_block_box"])
        .margin_bottom(10)
        .margin_top(10)
        .build();
    let code_block_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_bottom(10)
        .margin_end(10)
        .margin_start(10)
        .margin_top(7)
        .hexpand(false)
        .build();

    let syntax_opt = language_name.and_then(|l| {
        ps.find_syntax_by_token(l).or_else(|| {
            log::warn!("test");
            None
        })
    });
    let theme_opt = &ts.themes.get(highlight_theme_name);
    if theme_opt.is_none() {
        log::warn!("unknown theme name: {}", highlight_theme_name);
    }

    if let (Some(syntax), Some(theme)) = (syntax_opt, theme_opt) {
        let mut highlight_lines = HighlightLines::new(syntax, theme);
        let mut pango_str = String::new();
        for line in LinesWithEndings::from(content) {
            let ranges: Vec<(Style, &str)> = match highlight_lines.highlight_line(line, ps) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for (style, content) in ranges {
                let foreground = style.foreground;
                let (bold_start, bold_end) = if style
                    .font_style
                    .intersects(syntect::highlighting::FontStyle::BOLD)
                {
                    ("<b>", "</b>")
                } else {
                    ("", "")
                };
                let (italic_start, italic_end) = if style
                    .font_style
                    .intersects(syntect::highlighting::FontStyle::ITALIC)
                {
                    ("<i>", "</i>")
                } else {
                    ("", "")
                };
                let (underline_start, underline_end) = if style
                    .font_style
                    .intersects(syntect::highlighting::FontStyle::UNDERLINE)
                {
                    ("<u>", "</u>")
                } else {
                    ("", "")
                };

                pango_str.push_str(&format!(
                    "{}{}{}<span foreground=\"#{:02x?}{:02x?}{:02x?}\"><tt>{}</tt></span>{}{}{}",
                    bold_start,
                    italic_start,
                    underline_start,
                    foreground.r,
                    foreground.g,
                    foreground.b,
                    html2pango::html_escape(content),
                    bold_end,
                    italic_end,
                    underline_end,
                ));
            }
        }
        code_block_box.append(
            &gtk::Label::builder()
                .use_markup(true)
                .justify(gtk::Justification::Left)
                .halign(gtk::Align::Start)
                .selectable(true)
                .wrap(true)
                .focusable(false)
                .label(pango_str)
                .build(),
        );
    } else {
        code_block_box.append(
            &gtk::Label::builder()
                .use_markup(false)
                .justify(gtk::Justification::Left)
                .halign(gtk::Align::Start)
                .selectable(true)
                .wrap(true)
                .focusable(false)
                .label(content)
                .build(),
        );
    }

    outer_box.append(&code_block_box);
    root.append(&outer_box);
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_data(
        ".table_outer_box {
            background: darker(@theme_fg_color);
        }
        .table_inner_box {
            background: @theme_bg_color;
        }
        .code_block_box {
            background: @shade_color;
            border-radius: 10px;
        }",
    );

    // Add the provider to the default screen
    if let Some(display) = Display::default() {
        StyleContext::add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    } else {
        log::error!("unaleb to load CSS for commonmark renderer: could not connect to a display")
    }
}
