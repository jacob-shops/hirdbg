use serde::Deserialize;
use std;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use ratatui::text::Span;
use ratatui::style::Color;
use color_eyre;
use crossterm::event::{self, KeyCode, Event, KeyEventKind};
use similar;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HIRInstruction {
    // ptr: u32,
    // id: u32,
    opcode: String,
    // attributes: Vec<String>,
    // inputs: Vec<u32>,
    // uses: Vec<u32>,
    // mem_inputs: Vec<u32>,
    // #[serde(rename = "type")]
    // insn_type: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HIRBlock {
    // ptr: u32,
    // id: u32,
    // loop_depth: u32,
    // attributes: Vec<String>,
    // predecessors: Vec<u32>,
    // successors: Vec<u32>,
    instructions: Vec<HIRInstruction>,
}

#[derive(Clone, Deserialize)]
struct HIR {
    blocks: Vec<HIRBlock>
}

// #[derive(Clone, Deserialize)]
// struct LIRBlock {
    
// }

// #[derive(Clone, Deserialize)]
// struct LIR {
//     blocks: Vec<LIRBlock>
// }

#[derive(Clone, Deserialize)]
struct Pass {
    name: String,
    #[serde(rename = "mir")]
    hir: HIR,
    // lir: LIR,
}

#[derive(Clone, Deserialize)]
struct Iongraph {
    // name: String,
    passes: Vec<Pass>
}

struct TUIData {
    passes: Vec<Pass>,
    left: usize,
    right: usize,
    block: usize, // Note: the block index is always synced for both passes even one one side may not have a block
}

fn main() -> color_eyre::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }
    let file = std::fs::File::open(&args[1])?;
    let reader = std::io::BufReader::new(file);
    let json_data: Iongraph = serde_json::from_reader(reader)?;

    let mut tui_data = TUIData {
        passes: json_data.passes,
        left: 0,
        right: 1,
        block: 0
    };

    color_eyre::install()?;
    ratatui::run(|terminal| loop {
        terminal.draw(|frame| render(frame, &tui_data))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Left => {
                        if ((tui_data.left as isize) - 1 >= 0) && ((tui_data.right as isize) - 1 >= 0) {
                            tui_data.left -= 1;
                            tui_data.right -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if ((tui_data.left as isize) + 1 < tui_data.passes.len() as isize) && ((tui_data.right as isize) + 1 < tui_data.passes.len() as isize) {
                            tui_data.left += 1;
                            tui_data.right += 1;
                        }
                    }
                    KeyCode::Up => {
                        // TODO: Fix this because it ONLY considers the left pane and that's bad
                        // We probably want to allow this to look at "empty" blocks on either side
                        if (tui_data.block as isize) - 1 >= 0 {
                            tui_data.block -= 1;
                        }
                    }
                    KeyCode::Down => {
                        // TODO: Same as for "up"
                        if (tui_data.block as isize) + 1 < (tui_data.passes[tui_data.left].hir.blocks.len() as isize) {
                            tui_data.block += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    })
}

// fn apply_style_to_paragraph(lines: Vec<String>, styles: Vec<Style>) -> Vec<ratatui::text::Line<'_>> {
//     lines.into_iter()
//         .zip(styles).
//         map(|(line, style)| Line::styled(line, style))
//         .collect::<Vec<_>>()
// }

// TODO: This function needs to update both the original and changed strings, as well as apply styles for all of the changes for both
// Gross...
// fn apply_diff(original: &str, changed: &str) -> String {
//     let diff = similar::TextDiff::from_lines(original, changed);
//     let mut result: String = String::new();
//     for op in diff.ops() {
//         let sign = match change.tag() {
//             similar::ChangeTag::Delete => {"-"}
//             similar::ChangeTag::Insert => {"+"}
//             similar::ChangeTag::Equal => {" "}
//         };
//         result.push_str(&format!("{}{}", sign, change));
//     }
//     result
// }

// TODO: Don't draw the overall pane when there's no corresponding block on one side
fn render(frame: &mut Frame, data: &TUIData) {
    let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1);
    let horizontal = Layout::horizontal([Constraint::Percentage(50); 2]).spacing(1);
    let [top, main] = frame.area().layout(&vertical);
    let [left_area, right_area] = main.layout(&horizontal);

    let title = Line::from_iter([
        Span::from("HIR Dbg").bold(),
        Span::from(" (Press 'q' to quit)"),
    ]);
    frame.render_widget(title.centered(), top);

    let left_title = &data.passes[data.left].name;
    let right_title = &data.passes[data.right].name;

    let left_lines: Vec<&str> = data.passes[data.left].hir.blocks[data.block].instructions.iter().map(|insn| insn.opcode.as_str()).collect();
    let right_lines: Vec<&str> = data.passes[data.right].hir.blocks[data.block].instructions.iter().map(|insn| insn.opcode.as_str()).collect();

    let mut left_view: Vec<ratatui::text::Line<'_>> = Vec::new();
    let mut right_view: Vec<ratatui::text::Line<'_>> = Vec::new();
    // Do some fancy diff work to align things properly and highlight each side
    let diff = similar::TextDiff::from_slices(&left_lines, &right_lines);
    for op in diff.ops() {
        match *op {
            similar::DiffOp::Equal { old_index, new_index, len } => {
                for i in 0..len {
                    left_view.push(Line::styled(left_lines[old_index + i], Style::default()));
                    right_view.push(Line::styled(right_lines[new_index + i], Style::default()));
                }
            }
            similar::DiffOp::Delete { old_index, old_len, .. } => {
                for i in 0..old_len {
                    left_view.push(Line::styled(left_lines[old_index + i], Style::default().bg(Color::Red)));
                    right_view.push(Line::styled("", Style::default()));
                }
            }
            similar::DiffOp::Insert { new_index, new_len, .. } => {
                for i in 0..new_len {
                    left_view.push(Line::styled("", Style::default()));
                    right_view.push(Line::styled(right_lines[new_index + i], Style::default().bg(Color::Green)));
                }
            }
            similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
              let rows = old_len.max(new_len);
              for i in 0..rows {
                  let left = if i < old_len {
                      Line::styled(left_lines[old_index + i], Style::default().bg(Color::Red))
                  } else {
                      Line::styled("", Style::default())
                  };
                  let right = if i < new_len {
                      Line::styled(right_lines[new_index + i], Style::default().bg(Color::Green))
                  } else {
                      Line::styled("", Style::default())
                  };
                  left_view.push(left);
                  right_view.push(right);
              }
            }
        }
    };
    
    let left_widget = Paragraph::new(left_view).block(Block::bordered().title(left_title.clone()));
    let right_widget = Paragraph::new(right_view).block(Block::bordered().title(right_title.clone()));
    frame.render_widget(left_widget, left_area);
    frame.render_widget(right_widget, right_area);

}
