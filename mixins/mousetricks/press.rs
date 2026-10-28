// tweaks::Game::sort_mouse_press HEAD (pinned): a mouse press on any screen,
// before the game handles it. Mouse Tricks notes the drag it may start; the
// press is always the game's (and the sort key's).
fn mousetricks_on_press(game: &mut crate::game::Game<'_>, button: winit::event::MouseButton) {
    game.mousetricks_press(button);
}
