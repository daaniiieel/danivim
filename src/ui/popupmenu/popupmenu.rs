use std::sync::{Arc, Mutex};

use gdk;
use gtk;
use gtk::prelude::*;
use neovim_lib::neovim::Neovim;
use neovim_lib::neovim_api::NeovimApi;
use pango;

use nvim_bridge::{CompletionItem, PmenuColors};
use thread_guard::ThreadGuard;
use ui::common::calc_line_space;
use ui::common::{
    get_preferred_horizontal_position, get_preferred_vertical_position,
};
use ui::font::{Font, FontUnit};
use ui::ui::HlDefs;
use ui::popupmenu::CompletionItemWidgetWrap;
use ui::popupmenu::get_icon_pixbuf;
use ui::popupmenu::LazyLoader;

/// Maximum height of completion menu.
const MAX_HEIGHT: i32 = 500;
/// Fixed width of completion menu.
const DEFAULT_WIDTH_NO_DETAILS: i32 = 430;
const DEFAULT_WIDTH_WITH_DETAILS: i32 = 660;

struct State {

    items: LazyLoader,

    /// Size available for the popupmenu to use (width and height).
    available_size: Option<gdk::Rectangle>,
    /// Our anchor position where the popupmenu should be "pointing" to.
    anchor: gdk::Rectangle,

    current_width: i32,

    width_no_details: i32,
    width_with_details: i32,
}

impl State {
    fn new(list: gtk::ListBox, css_provider: gtk::CssProvider) -> Self {
        State {
            items: LazyLoader::new(list, css_provider),
            available_size: None,
            anchor: gdk::Rectangle {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },

            current_width: DEFAULT_WIDTH_NO_DETAILS,
            width_no_details: DEFAULT_WIDTH_NO_DETAILS,
            width_with_details: DEFAULT_WIDTH_WITH_DETAILS,
        }
    }
}

pub struct Popupmenu {
    /// Box that contains all the "content". This box is placed in side the
    /// layout container.
    box_: gtk::Box,
    /// Top level container.
    layout: gtk::Layout,
    /// Scrolled window that contains the list box that displays all the items.
    scrolled_list: gtk::ScrolledWindow,
    /// Scrolled window that contains the info label for full info view.
    scrolled_info: gtk::ScrolledWindow,
    /// List box that contains all the completion items.
    list: gtk::ListBox,
    /// Style provider for all internal widgets.
    css_provider: gtk::CssProvider,

    /// Flag telling if the info label is shown.
    info_shown: bool,
    /// Label for displaying full info of a completion item.
    info_label: gtk::Label,

    /// State that is in Arc because its passed into widget signal handlers.
    state: Arc<ThreadGuard<State>>,

    /// Our colors.
    colors: PmenuColors,
    /// Our font.
    font: Font,

    /// Line spacing.
    line_space: i64,
}

impl Popupmenu {
    /// Creates a new popupmenu.
    ///
    /// * `parent` - Overlay where popupmenu is placed. Ideally, this overlay
    ///              is where all the (neovim) grids are drawn.
    /// * `nvim` - Neovim instance. Popupmenu will instruct neovim to act on
    ///            user interaction.
    pub fn new(parent: &gtk::Overlay, nvim: Arc<Mutex<Neovim>>) -> Self {
        let css_provider = gtk::CssProvider::new();

        let info_label = gtk::Label::new("");
        info_label.set_halign(gtk::Align::Start);
        info_label.set_valign(gtk::Align::Start);
        info_label.set_xalign(0.0);
        info_label.set_line_wrap(true);
        info_label.set_line_wrap_mode(pango::WrapMode::WordChar);
        gtk::WidgetExt::set_name(&info_label, "info-label");

        // Because we're setting valign and halign to the info label, we'll
        // need to have some container in between the label and scrolled window.
        // Other wise there'll be some black boxes when scroll bars are needed.
        let info_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        info_box.add(&info_label);

        let scrolled_info = gtk::ScrolledWindow::new(None, None);
        scrolled_info.add(&info_box);
        scrolled_info
            .set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        let list = gtk::ListBox::new();
        list.set_valign(gtk::Align::Start);
        list.set_selection_mode(gtk::SelectionMode::Single);

        let scrolled_list = gtk::ScrolledWindow::new(None, None);
        scrolled_list.add(&list);
        scrolled_list
            .set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        box_.pack_start(&scrolled_list, true, true, 0);
        box_.pack_start(&scrolled_info, true, true, 0);
        box_.set_size_request(DEFAULT_WIDTH_NO_DETAILS, MAX_HEIGHT);
        box_.set_homogeneous(true);

        add_css_provider!(
            &css_provider,
            info_label,
            scrolled_info,
            list,
            scrolled_list,
            box_,
            // In gtk 3.18, the list box it self can't have borders,
            // so we'll have to add the border to its parent (which is the
            // viewport that scorlled window adds). This aint perfect,
            // but I didn't any find better solutions.
            scrolled_list.get_child().unwrap()
        );

        let state = Arc::new(ThreadGuard::new(State::new(list.clone(), css_provider.clone())));

        let state_ref = state.clone();
        let nvim_ref = nvim.clone();
        // When a row is activated (by mouse click), notify neovim to change
        // the selection to the activated row.
        list.connect_row_activated(move |_, row| {
            let state = state_ref.borrow_mut();
            let new = row.get_index();

            let selected = state.items.get_selected();

            let op = if new > selected {
                "<C-n>"
            } else {
                "<C-p>"
            };

            let mut payload = String::new();
            for _ in 0..(new - selected).abs() {
                payload.push_str(op)
            }

            let mut nvim = nvim_ref.lock().unwrap();
            nvim.input(payload.as_str()).unwrap();
        });

        let nvim_ref = nvim.clone();
        // On (mouse) button press...
        list.connect_button_press_event(move |_, e| {
            // ...check if the button press is double click.
            if e.get_event_type() == gdk::EventType::DoubleButtonPress {
                // And if so, tell neovim to select the current completion item.
                let mut nvim = nvim_ref.lock().unwrap();
                nvim.input("<C-y>").unwrap();
            }

            Inhibit(false)
        });

        // TODO(ville): Should use gtk::Fixed here.
        let layout = gtk::Layout::new(None, None);
        layout.put(&box_, 0, 0);
        layout.show_all();
        scrolled_info.hide();

        let state_ref = state.clone();
        layout.connect_size_allocate(move |_, alloc| {
            let mut state = state_ref.borrow_mut();
            state.available_size = Some(*alloc);
        });

        let state_ref = state.clone();
        let layout_ref = layout.clone();
        let scrolled_list_ref = scrolled_list.clone();
        let scrolled_info_ref = scrolled_info.clone();
        box_.connect_size_allocate(move |box_, alloc| {
            let state = state_ref.borrow();
            let layout = layout_ref.clone();

            if let Some(area) = state.available_size {
                let pos = state.anchor;

                let (x, width) = get_preferred_horizontal_position(
                    &area,
                    &pos,
                    state.current_width,
                );
                let (y, height) = get_preferred_vertical_position(
                    &area,
                    &pos,
                    alloc.height.min(MAX_HEIGHT),
                );

                layout.move_(box_, x, y);

                box_.set_size_request(width, height);

                // If we moved the popupmenu above the achor position, make
                // sure our contents are aligned to the bottom so there is not
                // cap between the achor and the content it self.
                if y < pos.y {
                    // Use get_child to get the viewport which is between
                    // the scrolled window and the actual widget that is
                    // inside it.
                    scrolled_list_ref
                        .get_child()
                        .unwrap()
                        .set_valign(gtk::Align::End);
                    scrolled_info_ref
                        .get_child()
                        .unwrap()
                        .set_valign(gtk::Align::End);
                } else {
                    scrolled_list_ref
                        .get_child()
                        .unwrap()
                        .set_valign(gtk::Align::Start);
                    scrolled_info_ref
                        .get_child()
                        .unwrap()
                        .set_valign(gtk::Align::Start);
                }
            }
        });

        parent.add_overlay(&layout);
        // Hide the layout initially so it wont catch any input events that
        // should go to the girds.
        layout.hide();

        Popupmenu {
            box_,
            layout,
            css_provider,
            list,
            scrolled_list,
            scrolled_info,
            info_label,
            state,
            info_shown: false,
            colors: PmenuColors::default(),
            font: Font::default(),
            line_space: 0,
        }
    }

    pub fn toggle_show_info(&mut self) {
        {
            let state = self.state.borrow();

            self.info_shown = !self.info_shown;

            let info_shown = self.info_shown;
            let info_label = self.info_label.clone();
            state.items.with_selected_item(|item| if let Some(item) = item {

                item.info.set_visible(!info_shown);
                item.menu.set_visible(!info_shown);

                if item.item.info.len() == 0 {
                    item.info.set_visible(false);
                }

                info_label.set_visible(
                    info_shown && item.item.menu.len() + item.item.info.len() > 0,
                );
            });

            if !self.info_shown {
                let adj = self.scrolled_info.get_vadjustment().unwrap();
                adj.set_value(0.0);
                // TODO(ville): There is a bug in GTK+ and some adjustment animations,
                //              where the adjustment's value is set back to upper - page-size
                //              if the user has "overshot" the scrolling. Work around this.
            }

            self.scrolled_info.set_visible(self.info_shown);
        }

        self.ensure_container_width();
    }

    fn ensure_container_width(&mut self) {
        let mut state = self.state.borrow_mut();

        state.current_width = if self.info_shown {
            state.width_with_details
        } else {
            state.width_no_details
        };

        self.box_.set_size_request(state.current_width, MAX_HEIGHT);
    }

    pub fn set_width(&mut self, w: i32) {
        {
            let mut state = self.state.borrow_mut();
            state.width_no_details = w;
        }
        self.ensure_container_width();
    }

    pub fn set_width_details(&mut self, w: i32) {
        {
            let mut state = self.state.borrow_mut();
            state.width_with_details = w;
        }
        self.ensure_container_width();
    }

    /// Hides the popupmenu.
    pub fn hide(&mut self) {
        self.layout.hide();
    }

    /// Shows the popupmenu.
    pub fn show(&self) {
        self.layout.show();
        self.box_.check_resize();
    }

    /// Sets the anchor point for popupmenu.
    pub fn set_anchor(&self, rect: gdk::Rectangle) {
        let mut state = self.state.borrow_mut();
        self.layout.move_(&self.box_, rect.x, rect.y + rect.height);
        state.anchor = rect;
    }

    pub fn set_items(&mut self, items: Vec<CompletionItem>, hl_defs: &HlDefs) {
        let mut state = self.state.borrow_mut();
        state.items.set_items(
            items,
            self.colors.fg.unwrap_or(hl_defs.default_fg),
        );

        self.list.show_all();
    }

    pub fn select(&mut self, item_num: i32, hl_defs: &HlDefs) {
        let mut state = self.state.borrow_mut();

        state.items.with_selected_item(|item| if let Some(item) = item {
            item.info.set_visible(false);
            item.menu.set_visible(false);

            // Update the `kind` icon with defualt fg color.
            let buf = get_icon_pixbuf(
                &item.item.kind,
                &self.colors.fg.unwrap_or(hl_defs.default_fg),
            );
            item.kind.set_from_pixbuf(&buf);
        });

        let prev = state.items.get_selected();
        state.items.select(item_num);
        let selected = state.items.get_selected();

        if selected < 0 {
            self.list.unselect_all();
            self.info_label.set_text("");
            self.info_label.hide();

            // If selecteion is removed, move the srolled window to the top.
            let adj = self.scrolled_list.get_vadjustment().unwrap();
            gtk::idle_add(move || {
                adj.set_value(0.0);
                Continue(false)
            });

            return;
        }

        state.items.with_selected_item(|item| if let Some(item) = item {
            item.info.set_visible(!self.info_shown);
            item.menu.set_visible(!self.info_shown);

            if item.item.info.len() == 0 {
                item.info.set_visible(false);
            }

            // Update the `kind` icon with "selected" fg color.
            let buf = get_icon_pixbuf(
                &item.item.kind,
                &self.colors.sel_fg.unwrap_or(hl_defs.default_fg),
            );
            item.kind.set_from_pixbuf(&buf);

            // If we went from no selection to state where the last item
            // is selected, we'll have to do some extra work to make sure
            // that the whole item is visible.
            let max = state.items.len() as i32 - 1;
            let adj = self.scrolled_list.get_vadjustment().unwrap();
            if prev == -1 && selected == max {
                adj.set_value(adj.get_upper());
            }

            let newline =
                if item.item.menu.len() > 0 && item.item.info.len() > 0 {
                    "\n"
                } else {
                    ""
                };

            self.info_label.set_text(&format!(
                "{}{}{}",
                item.item.menu, newline, item.item.info
            ));

            let has_info_content =
                item.item.menu.len() + item.item.info.len() > 0;
            self.info_label
                .set_visible(self.info_shown && has_info_content);
        });
    }

    pub fn set_colors(&mut self, colors: PmenuColors, hl_defs: &HlDefs) {
        self.colors = colors;
        self.set_styles(hl_defs);

        //let mut state = self.state.borrow_mut();
        //state.items.set_icon_colors(
            //self.colors.fg.unwrap_or(hl_defs.default_fg),
            //self.colors.sel_fg.unwrap_or(hl_defs.default_fg),
        //);
    }

    pub fn set_line_space(&mut self, space: i64, hl_defs: &HlDefs) {
        self.line_space = space;
        self.set_styles(hl_defs);

        // Set line space to the info_label with pango attrs.
        let attrs = pango::AttrList::new();
        let attr =
            pango::Attribute::new_rise(self.line_space as i32 * pango::SCALE)
                .unwrap();
        attrs.insert(attr);
        self.info_label.set_attributes(&attrs);
    }

    fn set_styles(&self, hl_defs: &HlDefs) {
        if gtk::get_minor_version() < 20 {
            self.set_styles_pre20(hl_defs);
        } else {
            self.set_styles_post20(hl_defs);
        }
    }

    fn set_styles_post20(&self, hl_defs: &HlDefs) {
        let (above, below) = calc_line_space(self.line_space);

        let css = format!(
            "{font_wild}

            grid, list, row, label {{
                color: #{normal_fg};
                background-color: #{normal_bg};
                outline: none;
            }}

            #info-label, list {{
                border: 1px solid #{normal_fg};
            }}

            row {{
                padding-top: {above}px;
                padding-bottom: {below}px;
            }}

            row:selected, row:selected > grid, row:selected > grid > label {{
                color: #{selected_fg};
                background-color: #{selected_bg};
            }}

            box {{
            }}
            ",
            font_wild = self.font.as_wild_css(FontUnit::Point),
            normal_fg = self.colors.fg.unwrap_or(hl_defs.default_fg).to_hex(),
            normal_bg = self.colors.bg.unwrap_or(hl_defs.default_bg).to_hex(),
            selected_bg =
                self.colors.sel_bg.unwrap_or(hl_defs.default_bg).to_hex(),
            selected_fg =
                self.colors.sel_fg.unwrap_or(hl_defs.default_fg).to_hex(),
            above = above.max(0),
            below = below.max(0),
        );
        CssProviderExt::load_from_data(&self.css_provider, css.as_bytes())
            .unwrap();
    }

    fn set_styles_pre20(&self, hl_defs: &HlDefs) {
        let (above, below) = calc_line_space(self.line_space);

        let css = format!(
            "{font_wild}

            GtkGrid, GtkListBox, GtkListBoxRow, GtkLabel {{
                color: #{normal_fg};
                background-color: #{normal_bg};
                outline: none;
            }}

            GtkViewport {{
                border-radius: 0px;
            }}

            #info-label, GtkViewport {{
                border: 1px solid #{normal_fg};
            }}

            GtkListBoxRow {{
                padding-top: {above}px;
                padding-bottom: {below}px;
            }}

            GtkListBoxRow:selected,
            GtkListBoxRow:selected > GtkGrid,
            GtkListBoxRow:selected > GtkGrid > GtkLabel {{
                color: #{selected_fg};
                background-color: #{selected_bg};
            }}
            ",
            font_wild = self.font.as_wild_css(FontUnit::Pixel),
            normal_fg = self.colors.fg.unwrap_or(hl_defs.default_fg).to_hex(),
            normal_bg = self.colors.bg.unwrap_or(hl_defs.default_bg).to_hex(),
            selected_bg =
                self.colors.sel_bg.unwrap_or(hl_defs.default_bg).to_hex(),
            selected_fg =
                self.colors.sel_fg.unwrap_or(hl_defs.default_fg).to_hex(),
            above = above.max(0),
            below = below.max(0),
        );
        CssProviderExt::load_from_data(&self.css_provider, css.as_bytes())
            .unwrap();
    }

    pub fn set_font(&mut self, font: Font, hl_defs: &HlDefs) {
        self.font = font;
        self.set_styles(hl_defs);
    }
}