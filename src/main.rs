mod actions;
mod global_handler;
mod homebridge;
mod models;
mod poller;
mod state;

use actions::{
    AdjustStateAction, BrightnessAction, HomebridgeDevicesAction, LaunchHomebridgeUiAction,
    SetStateAction, SwitchAction,
};
use global_handler::HomebridgeGlobalEventHandler;
use openaction::global_events::set_global_event_handler;
use openaction::{OpenActionResult, register_action, run};
use poller::spawn_state_poller;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use state::PluginState;

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    if let Err(error) = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Never,
    ) {
        eprintln!("Logger initialisation failed: {error}");
    }

    let state = PluginState::new().unwrap_or_else(|error| {
        panic!("Unable to initialise the OpenHomeB plugin: {error}")
    });

    let global_handler = Box::leak(Box::new(HomebridgeGlobalEventHandler::new(state.clone())));
    set_global_event_handler(global_handler);

    register_action(HomebridgeDevicesAction::new(state.clone())).await;
    register_action(SwitchAction::new(state.clone())).await;
    register_action(SetStateAction::new(state.clone())).await;
    register_action(AdjustStateAction::new(state.clone())).await;
    register_action(BrightnessAction::new(state.clone())).await;
    register_action(LaunchHomebridgeUiAction::new(state.clone())).await;

    spawn_state_poller(state);
    run(std::env::args().collect()).await
}
