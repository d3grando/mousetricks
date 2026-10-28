// Mouse Tricks' adapter to the game (Horizon 142388e1c136e622652ec7505af15d8ad195e041,
// see horizon-mixin.toml). Injected as `crate::game::mousetricks_native`; binds
// the game-independent drags and wheel moves in `crate::game::mousetricks`
// (runtime.rs) to the window's inventory screens.
//
// Every item it moves goes through the game's own container click path
// (`menu_click`: the verified menu engine in a world of this game's own, a
// predicted `ContainerClick` per click on a server or a LAN host), the clicks
// a hand makes, so a server accepts them as it accepts any player's.
use super::mousetricks::{
    self as mt, Button, Click, Config, Drag, Effect, Layout, Loaded, Menu, Moment, Press, Wheel,
};
use super::{Game, Screen};
use horizon_client::containers::BlockEntity;
use horizon_client::items::ItemStack;
use horizon_client::menu::Input;
use std::cell::RefCell;
use winit::event::MouseButton;

/// The window's state: mouse input only ever reaches the window's game, on
/// its own thread.
#[derive(Default)]
struct State {
    /// `config.json` as last read; None until something needs it.
    config: Option<Loaded>,
    /// The drag from the last press, until its release.
    drag: Option<Drag>,
    wheel: Wheel,
}
thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}
/// Never held while the game runs.
fn state<T>(run: impl FnOnce(&mut State) -> T) -> T {
    STATE.with(|s| run(&mut s.borrow_mut()))
}

/// `config.json` in the mod's settings folder; missing is the defaults.
fn read_config(game: &Game<'_>) -> Loaded {
    match game.native_setting(mt::ID, mt::CONFIG_FILE) {
        Some(text) => Config::from_json(&text).map_err(|e| format!("{}: {e}", mt::CONFIG_FILE)),
        None => Ok(Config::default()),
    }
}
/// Puts `loaded` in effect.
fn apply(loaded: Loaded) {
    if let Err(error) = &loaded {
        eprintln!("Mouse Tricks disabled: {error}");
    }
    state(|s| s.config = Some(loaded));
}
/// The configuration as read (once).
fn loaded(game: &Game<'_>) -> Loaded {
    if let Some(loaded) = state(|s| s.config.clone()) {
        return loaded;
    }
    let read = read_config(game);
    if let Err(error) = &read {
        eprintln!("Mouse Tricks disabled: {error}");
    }
    state(|s| s.config.get_or_insert(read).clone())
}
/// The configuration in effect; None when the file is broken (fails closed:
/// the screens click as the game's own).
fn config(game: &Game<'_>) -> Option<Config> {
    loaded(game).ok()
}
/// Not turned off in Options > Mods, in the window's game: every hook
/// passes through untouched otherwise (the list and settings hooks still
/// run, so the row can turn it back on).
fn on(game: &Game<'_>) -> bool {
    game.native_window() && game.native_on(mt::ID)
}

/// The open screen through the game's slots and clicks.
struct Native<'g, 'r> {
    game: &'g mut Game<'r>,
}
impl Menu for Native<'_, '_> {
    type Stack = ItemStack;
    fn slot(&mut self, slot: usize) -> Option<ItemStack> {
        self.game.slot_view(slot)
    }
    fn carried(&self) -> Option<ItemStack> {
        self.game.carried.clone()
    }
    fn count(&self, stack: &ItemStack) -> u32 {
        stack.count
    }
    fn max(&self, stack: &ItemStack) -> u32 {
        self.game.max_stack(stack).max(1)
    }
    fn same(&self, a: &ItemStack, b: &ItemStack) -> bool {
        a.same_item_same_components(b)
    }
    /// `AbstractContainerMenu.clicked` as a hand's click: the menu engine
    /// here, a predicted `ContainerClick` on a server (`menu_ops`).
    fn click(&mut self, slot: usize, click: Click) -> bool {
        let (button, input) = match click {
            Click::Left => (0, Input::Pickup),
            Click::Right => (1, Input::Pickup),
            Click::QuickMove => (0, Input::QuickMove),
        };
        let game = &mut *self.game;
        let done = game.menu_click(Some(slot), button, input);
        if done
            && game.remote.is_none()
            && let Some(key) = game.container
        {
            game.container_changed(key);
        }
        done
    }
    /// The game's quick craft drag and its release's click are dropped, as
    /// for a press the game handles itself (a double click's `PICKUP_ALL`),
    /// and so is Horizon's own Mouse Tweaks-style drag (its extra).
    fn take_over(&mut self) {
        let game = &mut *self.game;
        game.inv_drag = None;
        game.inv_clicks.skip_release();
        game.tweak_release();
    }
}

impl Game<'_> {
    /// The open screen is one Mouse Tricks works in: the inventory, or a
    /// chest-like container (chests, barrels, ender chests, shulker boxes,
    /// hoppers, dispensers), where every slot is a plain one. Never a
    /// station (furnace, anvil...), a mount's inventory or the creative one.
    fn mousetricks_screen(&self) -> bool {
        if self.screen != Screen::Inventory || self.station.is_some() || self.mount_active() {
            return false;
        }
        if self.remote.is_some() {
            self.remote_storage_open()
        } else {
            matches!(self.container_kind(), None | Some(BlockEntity::Chest(_)))
        }
    }
    /// The slot under the cursor (never the crafting result).
    fn mousetricks_hovered(&self) -> Option<usize> {
        let (slot, outside) = self.inventory_target();
        slot.filter(|&s| !outside && s != 200)
    }

    /// `tweaks::Game::sort_mouse_press`'s HEAD: a mouse press on any screen,
    /// before the game handles it. On an inventory screen it starts the drag
    /// the press may become; the press itself is always the game's.
    pub(super) fn mousetricks_press(&mut self, button: MouseButton) {
        state(|s| s.drag = None);
        if !on(self) || !self.mousetricks_screen() {
            return;
        }
        let button = match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            _ => return,
        };
        let Some(config) = config(self) else {
            return;
        };
        let press = Press {
            button,
            slot: self.mousetricks_hovered(),
            shift: self.has_shift_down(),
            holding: self.carried.is_some(),
        };
        let drag = Drag::start(&press, &config);
        state(|s| s.drag = drag);
    }

    /// `Game::inventory_drag_move`'s HEAD: true when the move belongs to
    /// Mouse Tricks' drag (the game's own drag doesn't grow).
    pub(super) fn mousetricks_drag_move(&mut self) -> bool {
        let Some(mut drag) = state(|s| s.drag.take()) else {
            return false;
        };
        if !on(self) || !self.mousetricks_screen() {
            return false;
        }
        let kind = drag.button().drag_type();
        let now = Moment {
            hovered: self.mousetricks_hovered(),
            shift: self.has_shift_down(),
            vanilla: self
                .inv_drag
                .as_ref()
                .filter(|(ty, _)| *ty == kind)
                .map(|(_, slots)| slots.clone()),
        };
        let owned = drag.moved(&mut Native { game: self }, &now);
        state(|s| s.drag = Some(drag));
        owned
    }

    /// `Game::inventory_mouse_release`'s HEAD: the drag ends. A drag the
    /// mod took over already dropped the game's, so the release clicks
    /// nothing more.
    pub(super) fn mousetricks_release(&mut self) {
        state(|s| s.drag = None);
    }

    /// `tweaks::Game::tweak_scroll`'s HEAD: true when the wheel moved items
    /// (or would have, over a stack it works on); false leaves the wheel to
    /// the game. A bundle under the cursor takes the wheel before this.
    pub(super) fn mousetricks_scroll(&mut self, delta: f32, slot: Option<usize>) -> bool {
        let usable = on(self) && self.mousetricks_screen() && self.carried.is_none();
        let Some(config) = config(self).filter(|c| usable && c.wheel) else {
            state(|s| s.wheel.reset());
            return false;
        };
        let layout = Layout {
            container: self
                .slots()
                .into_iter()
                .filter(|s| (300..500).contains(s))
                .collect(),
        };
        let Some(slot) = slot.filter(|&s| !layout.other_side(s).is_empty()) else {
            state(|s| s.wheel.reset());
            return false;
        };
        let steps = state(|s| s.wheel.steps(delta));
        mt::scroll(&mut Native { game: self }, &layout, &config, slot, steps);
        true
    }

    /// `native_frame` (a hook point): a drag or wheel left over from a
    /// screen that closed (its release went to another screen) ends.
    pub(super) fn mousetricks_frame(&mut self) {
        if self.screen != Screen::Inventory {
            state(|s| {
                s.drag = None;
                s.wheel.reset();
            });
        }
    }

    /// `native_settings` (a hook point): the settings page's buttons.
    pub(super) fn mousetricks_settings(&self) -> Vec<String> {
        mt::labels(&loaded(self))
    }

    /// `native_settings_click` (a hook point): change the setting, write
    /// `config.json` and put it in effect at once (or read the file again).
    pub(super) fn mousetricks_settings_click(&mut self, button: usize) {
        match mt::click(&loaded(self), button) {
            Some(Effect::Save(config)) => {
                self.native_set_setting(mt::ID, mt::CONFIG_FILE, &config.to_json());
                apply(Ok(config));
            }
            Some(Effect::Reload) => apply(read_config(self)),
            None => {}
        }
    }
}
