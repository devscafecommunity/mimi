//! Main REPL event loop

use std::io::{self, BufRead, Write};

use super::{PromptState, ReplConfig, ReplEditor, SpecialCommand};

pub async fn run_repl(config: ReplConfig) -> io::Result<()> {
    let mut editor = ReplEditor::new(config.max_history);
    let stdin = io::stdin();
    let reader = stdin.lock();
    let mut lines = reader.lines();

    let mut prompt_state = PromptState::Default;
    let mut multiline_buffer = String::new();

    run_startup_script(&config, &mut editor).await;

    print_prompt(prompt_state);

    while let Some(Ok(line)) = lines.next() {
        let line = line.trim();

        if line.is_empty() {
            print_prompt(prompt_state);
            continue;
        }

        if line.ends_with('\\') {
            multiline_buffer.push_str(&line[..line.len() - 1]);
            multiline_buffer.push(' ');
            prompt_state = PromptState::Continuation;
            print_prompt(prompt_state);
            continue;
        }

        if !multiline_buffer.is_empty() {
            multiline_buffer.push_str(line);
        } else {
            multiline_buffer = line.to_string();
        }

        editor.add_to_history(&multiline_buffer);

        match execute_line(&multiline_buffer, &mut editor) {
            LineExecution::Exit => break,
            LineExecution::Continue => {},
        }

        multiline_buffer.clear();
        prompt_state = PromptState::Default;
        print_prompt(prompt_state);
    }

    println!();
    Ok(())
}

enum LineExecution {
    Exit,
    Continue,
}

fn execute_line(line: &str, editor: &mut ReplEditor) -> LineExecution {
    if let Some(cmd) = SpecialCommand::parse(line) {
        let output = cmd.execute();

        if cmd == SpecialCommand::Exit {
            return LineExecution::Exit;
        }

        if cmd == SpecialCommand::Clear {
            print!("{}", output);
            let _ = io::stdout().flush();
        } else if matches!(cmd, SpecialCommand::History(_)) {
            let history_lines = editor.get_history(20);
            for (i, h_line) in history_lines.iter().enumerate() {
                println!("  {}: {}", i + 1, h_line);
            }
        } else {
            println!("{}", output);
        }
    } else {
        println!(
            "REPL: parsed command '{}' (execution deferred to M1.4.6)",
            line
        );
    }

    LineExecution::Continue
}

fn print_prompt(state: PromptState) {
    print!("{}", state.prompt_string());
    let _ = io::stdout().flush();
}

async fn run_startup_script(config: &ReplConfig, editor: &mut ReplEditor) {
    if let Some(script_path) = &config.startup_script {
        if let Ok(content) = std::fs::read_to_string(script_path) {
            for script_line in content.lines() {
                let trimmed = script_line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    println!("{}{}", PromptState::Default.prompt_string(), trimmed);
                    editor.add_to_history(trimmed);
                    match execute_line(trimmed, editor) {
                        LineExecution::Exit => break,
                        LineExecution::Continue => {},
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_execution_exit() {
        let mut editor = ReplEditor::new(1000);
        let result = execute_line(".exit", &mut editor);
        assert!(matches!(result, LineExecution::Exit));
    }

    #[test]
    fn test_line_execution_regular() {
        let mut editor = ReplEditor::new(1000);
        let result = execute_line("some command", &mut editor);
        assert!(matches!(result, LineExecution::Continue));
    }

    #[test]
    fn test_print_prompt_default() {
        print_prompt(PromptState::Default);
    }
}
