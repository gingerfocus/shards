use crossterm::event::{KeyCode, KeyModifiers};
use reedline::{
    ColumnarMenu, DefaultCompleter, DefaultPrompt, ExampleHighlighter, FileBackedHistory, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, Vi, DefaultHinter, DefaultValidator,
};
use nu_ansi_term::{Color, Style};

pub struct Line {
    rl: Reedline,
    prompt: DefaultPrompt,
}

impl Line {
    pub fn new() -> Line {

        let commands = vec![
            "test".into(),
            "hello world".into(),
            "hello world reedline".into(),
            "this is the reedline crate".into(),
        ];

        let completer = Box::new(DefaultCompleter::new_with_wordlen(commands.clone(), 2));
        let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));


        let mut insert = reedline::default_vi_insert_keybindings();
        insert.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("completion_menu".to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );

        let validator = Box::new(DefaultValidator);

        let rl = reedline::Reedline::create()
            .with_edit_mode(Box::new(Vi::new(
                insert,
                reedline::default_vi_normal_keybindings(),
            )))
            // .with_history(Box::new(
            //     FileBackedHistory::with_file(5, "history.txt".into())
            //         .expect("Error configuring history with file"),
            // ))
            .with_highlighter(Box::new(ExampleHighlighter::new(commands)))
            .with_completer(completer)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
            .with_hinter(Box::new(
                DefaultHinter::default().with_style(Style::new().italic().fg(Color::Red)),
            ))
            .with_validator(validator);

        let prompt = reedline::DefaultPrompt::default();

        Self { rl, prompt }
    }

    pub fn next_line(&mut self) -> Option<String> {
        let sig = self.rl.read_line(&self.prompt);
        match sig {
            Ok(Signal::Success(b)) => Some(b),
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => None,
            x => {
                println!("Event: {:?}", x);
                None
            }
        }
    }
}
