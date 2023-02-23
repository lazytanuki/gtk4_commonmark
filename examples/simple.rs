use gtk4_commonmark::{render_input, RenderConfig};
use relm4::{adw, gtk, RelmApp, RelmContainerExt};

const INPUT_MARKDOWN: &str = include_str!("input.md");

struct Model {}
impl relm4::SimpleComponent for Model {
    type Input = ();
    type Output = ();
    type Init = ();
    type Root = adw::Window;
    type Widgets = ();

    fn init_root() -> Self::Root {
        adw::Window::new()
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        _sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // Parse the input and get a `gtk::Viewport` in return
        let viewport = render_input(INPUT_MARKDOWN, RenderConfig::default()).unwrap();

        // Scrollable area to put the viewport in, with a clamp to limit its width
        let scrollable = gtk::ScrolledWindow::new();
        let clamp = adw::Clamp::builder().maximum_size(800).build();
        clamp.container_add(&scrollable);
        scrollable.container_add(&viewport);
        root.container_add(&clamp);

        relm4::ComponentParts {
            model: Model {},
            widgets: (),
        }
    }
}

#[tokio::main]
async fn main() {
    // Init logger
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .expect("failed to initialize termlogger");

    // Create and display app
    let app = RelmApp::new("gtk4_commonmark");
    relm4::set_global_css(
        ".code_block_box {
                background: @shade_color;
                border-radius: 10px;
            }",
    );
    app.run::<Model>(());
}
