use crate::{Shell, ShellState};
use rush_core::{lexer::Lexer, walker::Expand};

pub fn expand(thing: Expand, state: &ShellState) -> String {
    match thing {
        Expand::Literal(s) => s,
        Expand::Var(k) => {
            log::info!("explanding key: {}", k);
            // let (var, rest) = state.get_env(&k);
            // format!("{}{}", var, rest)

            state.get_env_exact(&k).unwrap_or_default()
        }
        Expand::Home => state.home().to_owned(),
        // Expand::Brace(_, _, _) => todo!(),
        Expand::Sub(s) => {
            let s = Shell::sourced(Lexer::new(crate::util::OwnedCharBuffer::new(s)));
            s.run(false).unwrap();
            todo!("get shell output")
        }
        Expand::Brace(_, _, _) => todo!(),
    }
}
//     fn expand_word(&mut self, expansions: Vec<Expand>) -> String {
//         let mut phrase = String::new();
//         for word in expansions {
//             match word {
//                 Literal(s) => phrase.push_str(&s),
//                 Tilde(word) => {
//                     let s = self.expand_word(word);
//                     if s.is_empty() || s.starts_with('/') {
//                         phrase.push_str(&env::var("HOME").unwrap());
//                         phrase.push_str(&s);
//                     } else {
//                         let mut strings = s.splitn(1, '/');
//                         let name = strings.next().unwrap();
//                         if let Some(user) = User::from_name(name).unwrap() {
//                             phrase.push_str(user.dir.as_os_str().to_str().unwrap());
//                             if let Some(path) = strings.next() {
//                                 phrase.push_str(path);
//                             }
//                         } else {
//                             phrase.push('~');
//                             phrase.push_str(name);
//                         }
//                     }
//                 }
//                 Var(s) => {
//                     phrase.push_str(&self.shell.borrow().get_var(&s).unwrap_or_default());
//                 }
//                 Brace(key, action, word) => {
//                     let val = self.shell.borrow().get_var(&key);
//                     match action {
//                         Action::UseDefault(null) => {
//                             if let Some(s) = val {
//                                 if s == "" && null {
//                                     phrase.push_str(&self.expand_word(word))
//                                 } else {
//                                     phrase.push_str(&s)
//                                 }
//                             } else {
//                                 phrase.push_str(&self.expand_word(word))
//                             }
//                         }
//                         Action::AssignDefault(null) => {
//                             if let Some(s) = val {
//                                 if s == "" && null {
//                                     let expanded = self.expand_word(word);
//                                     phrase.push_str(&expanded);
//                                     self.shell.set_var(key, expanded);
//                                 } else {
//                                     phrase.push_str(&s)
//                                 }
//                             } else {
//                                 let expanded = self.expand_word(word);
//                                 phrase.push_str(&expanded);
//                                 self.shell.set_var(key, expanded);
//                             }
//                         }
//                         Action::IndicateError(null) => {
//                             if let Some(s) = val {
//                                 if s == "" && null {
//                                     let message = self.expand_word(word);
//                                     if message.is_empty() {
//                                         eprintln!("rush: {}: parameter null", key);
//                                     } else {
//                                         eprintln!("rush: {}: {}", key, message);
//                                     }
//                                     if !self.shell.is_interactive() {
//                                         exit(1);
//                                     }
//                                 } else {
//                                     phrase.push_str(&s)
//                                 }
//                             } else {
//                                 let message = self.expand_word(word);
//                                 if message.is_empty() {
//                                     eprintln!("rush: {}: parameter not set", key);
//                                 } else {
//                                     eprintln!("rush: {}: {}", key, message);
//                                 }
//                                 if !self.shell.is_interactive() {
//                                     exit(1);
//                                 }
//                             }
//                         }
//                         Action::UseAlternate(null) => {
//                             if let Some(s) = val {
//                                 if s != "" || !null {
//                                     phrase.push_str(&self.expand_word(word))
//                                 }
//                             }
//                         }
//                         Action::RmSmallestSuffix => todo!(),
//                         Action::RmLargestSuffix => todo!(),
//                         Action::RmSmallestPrefix => todo!(),
//                         Action::RmLargestPrefix => todo!(),
//                         Action::StringLength => todo!(),
//                     }
//                 }
//                 Sub(e) => {
//                     todo!("{:?}", e)
//                     // // FIXME: `$(ls something)`, commands with params don't work atm
//                     // // for some reason
//                     //
//                     // let mut parser = Parser::new(vec!(Word(e)).into_iter(), Rc::clone(&self.shell));
//                     //
//                     // // This setup here allows me to do a surprisingly easy subshell.
//                     // // Though subshells typically seem to inherit everything I'm keeping in my
//                     // // `shell` variable at the moment?
//                     // if let Ok(command) = parser.get() {
//                     //     #[cfg(debug_assertions)] // Only include when not built with `--release` flag
//                     //     println!("\u{001b}[33m{:#?}\u{001b}[0m", command);
//                     //
//                     //     let mut output = Runner::new(Rc::clone(&parser.shell)).execute(command, true).unwrap();
//                     //     output = output.replace(char::is_whitespace, " ");
//                     //     phrase.push_str(output.trim());
//                     // }
//                 }
//             }
//         }
//         phrase
//     }
