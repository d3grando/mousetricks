// native_hooks::Game::native_frame (a hook point): a drag left over from a
// closed screen ends.
fn mousetricks_frame(game: &mut crate::game::Game<'_>) {
    game.mousetricks_frame();
}
