# Mouse Tricks

A **native** [Horizon](https://github.com/horizon-rust/horizon) mod (a horizon-mixins source bundle, `.hnmod`) that
makes the mouse quicker in inventories: drag with the right button to drop one item into each slot, drag with the left
button to gather a stack's item from many slots or to Shift-move many stacks at once, and turn the wheel over a stack
to move its items one by one between your inventory and a container. Each trick can be turned off in game: Options >
Mods > Mouse Tricks > Settings.

Inspired by Mouse Tweaks (https://modrinth.com/mod/mouse-tweaks); not affiliated, no code from it. Mouse Tricks was
written from scratch for Horizon from Mouse Tweaks' public description only.

> **Native mods are not sandboxed.** Unlike `.hmod` (Wasm) mods, a native mod is Rust source compiled into your own
> Horizon executable and runs with your user rights. Read the source (it is small: `mixins/mousetricks/`) before
> building.

## What it does

Every trick is made of **ordinary slot clicks**, the very clicks your hand would make (left click, right click, Shift
click), sent through the game's own container click path. In a world of this game's own they run on the game's
verified menu engine; on a server (or a LAN host) each one is a normal `ContainerClick`, predicted on your side and
checked by the server like any click. Nothing is moved that a player couldn't move by clicking, so servers accept it,
and Mouse Tricks needs nothing on the server.

| Trick | How | Setting |
| --- | --- | --- |
| **Right-drag** | Holding a stack, press the right button over a slot and drag: as soon as the cursor leaves the first slot, it and every slot you enter get one item, at once (not at the release, as in vanilla). Entering a slot again puts one more in it. Slots holding another item are skipped (never swapped); full slots too. A right click that stays on its slot is the game's own click (one item, or a swap). | `rightDrag` |
| **Left-drag: gather** | With a stack in hand (or one the press just picked up), drag with the left button over slots of the **same item**: each one joins the stack in your hand, as long as all of it fits. Hold **Shift** while dragging and those slots are Shift-clicked (moved to the other side) instead. If your drag first spreads over two slots the vanilla way (to split the stack evenly), it stays vanilla's. | `leftDrag` |
| **Shift + left-drag: move** | With an empty hand, hold Shift, press the left button and drag: every stack you pass over is Shift-clicked, once per drag. Let go of Shift to pause. | `shiftDrag` |
| **Wheel** | With an empty hand, turn the wheel over a stack: **down** sends one of its items to the other side (onto a stack of it with room, else into the first empty slot, as a Shift click fills them), **up** brings one of the same item back into the stack. The other side is the container when one is open; in your own inventory, the hotbar and the main inventory trade. A bundle under the cursor keeps its own wheel. | `wheel` |

Where: the player's inventory screen (crafting grid and armour slots included for the drags) and the chest-like
containers whose slots are all plain ones (the same set Horizon's own inventory extra works in): chests, barrels,
ender chests and shulker boxes, in single player (whose window is a client of its own server thread), on LAN and on
servers; with `--in-frame` single player also hoppers, dispensers and the crafting table. Other screens (furnaces,
anvils, villagers, a horse's inventory, the creative inventory) click exactly as before.

## Settings

**Options > Mods** lists Mouse Tricks with its **Settings** button. The row's own button turns the whole mod off (it
then does nothing at all: every hook passes through until the launcher's next build leaves it out) and on again. On
the settings page each click moves a button to its next value; it applies at once.

| Button | Key | Values a click cycles through |
| --- | --- | --- |
| Right-drag: one item per slot: ON | `rightDrag` | ON, OFF |
| Left-drag: gather same items: ON | `leftDrag` | ON, OFF |
| Shift + left-drag: move stacks: ON | `shiftDrag` | ON, OFF |
| Wheel: move items one by one: ON | `wheel` | ON, OFF |
| Wheel direction: down sends, up brings back | `wheelDirection` | `normal`; `inverted` (up sends, down brings back); `positional` (items follow the wheel: up over the inventory sends into the container above it, down over the container sends down into the inventory; the hotbar sends up into the main inventory). Shown while the wheel is on |
| Wheel brings back from: last slot | `wheelSearch` | `lastToFirst`, `firstToLast`: which end of the other side the wheel takes an item from first. Shown while the wheel is on |
| Reset to defaults | | every setting back to its default |

The page writes `horizon-mods/.horizon-mod-data/mousetricks/config.json` (beside the game's `options.txt`):

```json
{
  "rightDrag": true,
  "leftDrag": true,
  "shiftDrag": true,
  "wheel": true,
  "wheelDirection": "normal",
  "wheelSearch": "lastToFirst"
}
```

Every key is optional. A file that doesn't parse (an unknown key, a wrong value) turns every trick off: the page shows
the error (click it to read the file again) and a reset button, and stderr gets `Mouse Tricks disabled: ...`.

### Horizon's own inventory extra

Horizon has a built-in, off-by-default extra with similar controls (Options > Horizon Settings... > Extras... >
"Mouse Tweaks controls"). Leave it off
while Mouse Tricks is on. If both are on, Mouse Tricks handles left drags and the wheel first, and Horizon's extra
keeps the right-drag (it takes the press before Mouse Tricks sees a drag), so the right-drag follows Horizon's rules,
not this mod's.

## Compatibility

Horizon's native mods can use **hook points** (`game/native_hooks.rs`, `NATIVE_HOOKS_API`): functions kept stable from
release to release. Mouse Tricks uses them for everything they cover:

- `native_mods`, `native_settings`, `native_settings_click`: the row in Options > Mods and the settings page;
  `native_setting` / `native_set_setting` / `native_on` / `native_window` for the file and the off switch;
- `native_frame`: a drag whose release never reached the inventory (the screen closed with the button down) is
  dropped.

The hook points pass mouse buttons (`native_key`) and the wheel (`native_wheel`) **only while playing**, never over an
inventory screen, so Mouse Tricks hooks four game functions directly and is **pinned** to one Horizon commit:

| Hook | Kind | Why it is not a hook point |
| --- | --- | --- |
| `tweaks::Game::sort_mouse_press` | HEAD, not cancellable | Notes the drag a press may start. It is the first call every mouse press over a screen makes; the press itself always goes on to the game. `inventory_press`, the real press handler, holds a `vec!` and horizon-mixins refuses macro bodies. |
| `Game::inventory_drag_move` | HEAD, cancellable | The cursor moving over an inventory screen: a move that belongs to a Mouse Tricks drag is cancelled so the game's quick craft drag doesn't grow; any other move goes on. |
| `Game::inventory_mouse_release` | HEAD, not cancellable | The button went up: the drag ends. The release goes on to the game (a drag Mouse Tricks took over already told the game's click state to skip its release, as the game does after a double click). |
| `tweaks::Game::tweak_scroll` | HEAD, cancellable | The wheel over an inventory screen, after a bundle had its turn: cancelled (answering "used") when Mouse Tricks moved items, else the game goes on. |

Every item Mouse Tricks moves goes through `Game::menu_click`, the game's own click path (no hook there).

Pinned: Horizon `142388e1c136e622652ec7505af15d8ad195e041`, horizon-mixins `0d6b103c70633d8fd9443d8d0acd3aadaacedb06`,
game 26.3: the same pins as TreeFall 0.3.0 and Mason 0.2.0, so the three build together into one game (checked:
`horizon-mixins plan` and `apply` of all three on that commit, 24 injections, 14 modules, no conflict). None of
Mouse Tricks' pinned targets is hooked by either of them.

## Build and try

Mouse Tricks is a source bundle; the launcher compiles it into a custom game:

```sh
horizon-launcher native-build <this folder> --version v0.2.97 --trust-native-code
```

`<this folder>` may also be a folder holding several native mods.

## Development

`mixins/mousetricks/`:

- `horizon-mixin.toml`: two support modules and eight HEAD hooks (above).
- `runtime.rs`: everything the tricks decide (when a drag becomes the mod's, which slots take an item, the wheel's
  targets and sources, the settings file and page) as pure code over a small `Menu` trait, with unit tests against a
  fake menu that clicks as the game's menus do. It is both this repository's Cargo library and the injected
  `crate::game::mousetricks` module.
- `native.rs`: the adapter to the game (`crate::game::mousetricks_native`): the game's slots, carried stack and clicks.
- `press.rs`, `drag.rs`, `release.rs`, `wheel.rs`, `frame.rs`, `list.rs`, `settings.rs`, `click.rs`: the hook helpers.

```sh
cargo test
cargo fmt --check
rustfmt --edition 2024 --check mixins/mousetricks/*.rs
cargo clippy --all-targets -- -D warnings
python tools/pack_hnmod.py   # dist/mousetricks-<version>.hnmod
```

MIT licensed, (c) 2026 d3grando.
