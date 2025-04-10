use druid::widget::{Flex, Label, Button, TextBox, List, Scroll};
use druid::{
    AppLauncher, Widget, WindowDesc, Data, Lens, Env, Event, EventCtx, TimerToken, Command,
    Selector, AppDelegate, DelegateCtx, Target, WidgetExt,
};
use druid::im::Vector;
use serde::{Serialize, Deserialize};
use std::time::Duration;

// Custom Command for selecting a task
const SELECT_TASK: Selector<String> = Selector::new("select_task");

/// Represents a single task with a name and accumulated time in seconds.
#[derive(Clone, Data, Lens, Serialize, Deserialize)]
struct Task {
    name: String,
    accumulated: u64,
}

/// Holds all tasks and other UI fields.
#[derive(Clone, Data, Lens, Serialize, Deserialize)]
struct AppState {
    tasks: Vector<Task>,
    selected: Option<usize>,
    new_task_name: String,
    #[serde(skip)]
    #[data(ignore)]
    timer_token: Option<TimerToken>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            tasks: Vector::new(),
            selected: None,
            new_task_name: "".to_string(),
            timer_token: None,
        }
    }
}

/// Helper function to convert seconds to HH:MM:SS format.
fn format_time(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Builds the UI layout for the application.
fn build_ui() -> impl Widget<AppState> {
    // Row with a TextBox and an "Add Task" button.
    let input_row = Flex::row()
        .with_child(TextBox::new().lens(AppState::new_task_name).fix_width(200.0))
        .with_child(
            Button::new("Add Task").on_click(|ctx, data: &mut AppState, _env| {
                if !data.new_task_name.trim().is_empty() {
                    data.tasks.push_back(Task {
                        name: data.new_task_name.clone(),
                        accumulated: 0,
                    });
                    data.new_task_name.clear();
                    save_state(data);
                    ctx.request_update();
                }
            }),
        );
    
    // Button to remove the currently selected task.
    let remove_button = Button::new("Remove Selected Task").on_click(|ctx, data: &mut AppState, _env| {
        if let Some(idx) = data.selected {
            data.tasks.remove(idx);
            data.selected = None;
            save_state(data);
            ctx.request_update();
        }
    });
    
    // Create a list widget for the tasks.
    let task_list = List::new(|| {
        Flex::row()
            .with_child(Label::new(|task: &Task, _env: &Env| task.name.clone()).fix_width(150.0))
            // Display the accumulated time in HH:MM:SS format.
            .with_child(Label::new(|task: &Task, _env: &Env| format_time(task.accumulated)).fix_width(100.0))
            .with_spacer(10.0)
            .with_child(Button::new("Select").on_click(|ctx, task: &mut Task, _env| {
                // Submit a command with the task name so that the AppDelegate can update
                // which task is currently selected.
                ctx.submit_command(SELECT_TASK.with(task.name.clone()));
            }))
    })
    .lens(AppState::tasks);
    
    let scrollable_list = Scroll::new(task_list).vertical();

    // Assemble the complete layout.
    Flex::column()
        .with_child(input_row)
        .with_spacer(8.0)
        .with_child(remove_button)
        .with_spacer(8.0)
        .with_child(scrollable_list)
        // Attach a controller to handle timer events.
        .controller(TimerController {})
}

/// Sets up a repeating timer event.
struct TimerController;

impl<W: Widget<AppState>> druid::widget::Controller<AppState, W> for TimerController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        match event {
            // When the window connects, start the timer.
            Event::WindowConnected => {
                let token = ctx.request_timer(Duration::from_secs(1));
                data.timer_token = Some(token);
            },
            // On each timer tick, if a task is selected, update its time.
            Event::Timer(token) => {
                if Some(*token) == data.timer_token {
                    if let Some(idx) = data.selected {
                        if let Some(task) = data.tasks.get_mut(idx) {
                            task.accumulated += 1;
                        }
                        save_state(data);
                        ctx.request_update();
                    }
                    // Request the next tick in 1 second.
                    let token = ctx.request_timer(Duration::from_secs(1));
                    data.timer_token = Some(token);
                }
            },
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}

/// An application delegate that listens for the custom "Select" command.
struct Delegate;

impl AppDelegate<AppState> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppState,
        _env: &Env,
    ) -> druid::Handled {
        if let Some(task_name) = cmd.get(SELECT_TASK) {
            if let Some((idx, _)) = data.tasks.iter().enumerate().find(|(_, task)| &task.name == task_name) {
                data.selected = Some(idx);
                return druid::Handled::Yes;
            }
        }
        druid::Handled::No
    }
}

/// Saves the application state to a file called "tasks.json".
fn save_state(state: &AppState) {
    if let Ok(json) = serde_json::to_string(state) {
        if let Err(e) = std::fs::write("tasks.json", json) {
            println!("Failed to save state: {}", e);
        }
    }
}

/// Attempts to load the application state from "tasks.json".
fn load_state() -> AppState {
    if let Ok(data) = std::fs::read_to_string("tasks.json") {
        if let Ok(state) = serde_json::from_str::<AppState>(&data) {
            return state;
        }
    }
    AppState::new()
}

fn main() {
    // Create a window with our UI.
    let main_window = WindowDesc::new(build_ui()).title("Task Tracker");
    // Load the initial state (or create a new one if not available).
    let initial_state = load_state();
    // Launch the application with our delegate.
    AppLauncher::with_window(main_window)
        .delegate(Delegate {})
        .launch(initial_state)
        .expect("Failed to launch application");
}
