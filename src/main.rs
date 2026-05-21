use serde::Deserialize;
use std;
use ratatui::prelude::*;
use ratatui::layout::{Flex};
use ratatui::widgets::{Block, Paragraph, Borders};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::style::Color;
use color_eyre;
use crossterm::event::{self, KeyCode, Event, KeyEventKind, KeyModifiers};
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
    function: usize,
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
        function: 0,
        block: 0,
    };

    color_eyre::install()?;
    ratatui::run(|terminal| loop {
        terminal.draw(|frame| render(frame, &tui_data))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::NONE) => break Ok(()),
                    // Slide the entire diffing window left
                    (KeyCode::Left, KeyModifiers::NONE) => {
                        if (tui_data.left > 0) && (tui_data.right > 0) {
                            tui_data.left -= 1;
                            tui_data.right -= 1;
                        }
                    }
                    // Slide the entire diffing window right
                    (KeyCode::Right, KeyModifiers::NONE) => {
                        if (tui_data.left + 1 < tui_data.passes.len()) && (tui_data.right + 1 < tui_data.passes.len()) {
                            tui_data.left += 1;
                            tui_data.right += 1;
                        }
                    }
                    // Shrink the size of the diffing window (by modifying the right pane)
                    (KeyCode::Left, KeyModifiers::SHIFT) => {
                        if tui_data.left + 1 < tui_data.right {
                            tui_data.right -= 1;
                        }
                    }
                    // Grow the size of the diffing window (by modifying the right pane)
                    (KeyCode::Right, KeyModifiers::SHIFT) => {
                        if tui_data.right + 1 < tui_data.passes.len() {
                            tui_data.right += 1;
                        }
                    }
                    (KeyCode::Up, KeyModifiers::NONE) => {
                        // FIX: This because it ONLY considers the left pane and that's bad when the right pane has more blocks
                        // We probably want to allow this to look at "empty" blocks on either side
                        if tui_data.block > 0 {
                            tui_data.block -= 1;
                        }
                    }
                    (KeyCode::Down, KeyModifiers::NONE) => {
                        // FIX: Same as for "up"
                        if tui_data.block + 1 < tui_data.passes[tui_data.left].hir.blocks.len() {
                            tui_data.block += 1;
                        }
                    }
                    (_, _) => {}
                }
            }
        }
    })
}

// TODO: Redesign so that opening a single iongraph file is a "special case" of comparing two.
// TODO: Figure out how to add tree sitter support for syntax highlighting
// In this special case, we use the custom rules about how the right pass must be greater than the left.
// In the general case, we compare two different files and the right pane has no such restriction. Additionally,
// - the titles of each block are different (representing the branch or something from which they come)
// - the horizontal div showing "how many passes" we are looking at gets replaced with simpler names and arrows on either side showing what we can navigate to
// TODO: When this is done, we need to improve the usage description to take one or two iongraph directories.
// TODO: Don't draw the overall pane when there's no corresponding block on one side
// TODO: We need a way to cycle between functions. This requires:
// - finding the function representation in iongraph json (each JSON file in the iongraph represents another function. Right now the names are mangled)
// - figuring out how we want to render where the functions are (a tab above the pass visualization?)
// - adding a mechanism to cycle between functions (press tab)
// - adding instructions along the bottom to explain function cycling (done)
// - Change program to load a set of iongraph JSONs by passing the directory path rather than an individual file
// TODO: Add help menu
fn render(frame: &mut Frame, data: &TUIData) {
    let vertical = Layout::vertical([Constraint::Percentage(10), Constraint::Fill(0)]);
    let horizontal = Layout::horizontal([Constraint::Percentage(50); 2]);
    let [top, main] = frame.area().layout(&vertical);
    let [left_area, right_area] = main.layout(&horizontal);

    let header_block = Block::bordered().title("HIR Dbg").title_alignment(Alignment::Center).borders(Borders::ALL);
    let inner = header_block.inner(top);
    frame.render_widget(header_block, top);

    let active_pass_names: Vec<&str> = data.passes[data.left+1..data.right+1].iter().map(|pass| pass.name.as_str()).collect();
    let row_elements: Vec<&str> = std::iter::empty()
        .chain(if data.left != 0 { Some("←")} else { None })                      // If we can move to the left, indicate with an arrow
        .chain(active_pass_names)                                                 // Collect all active pass names  
        .chain(if data.right != data.passes.len() - 1 { Some("→")} else { None }) // If we can move to the right, indicate with an arrow
        .collect();
    
    let constraints: Vec<Constraint> = row_elements
      .iter()
      .map(|elem| Constraint::Length(elem.len() as u16))
      .collect();
    let cells = Layout::horizontal(constraints)
      .flex(Flex::Center)
      .spacing(3)
      .split(inner);

    for (cell, label) in cells.iter().zip(row_elements) {
      frame.render_widget(Paragraph::new(label).centered(), *cell,);
    }

    let left_title = "Old";
    let right_title = "New";

    //TODO: Add a way to display which block we are accessing

    let left_lines: Vec<&str> = data.passes[data.left].hir.blocks[data.block].instructions.iter().map(|insn| insn.opcode.as_str()).collect();
    let right_lines: Vec<&str> = data.passes[data.right].hir.blocks[data.block].instructions.iter().map(|insn| insn.opcode.as_str()).collect();

    let mut left_view: Vec<ratatui::text::Line<'_>> = Vec::new();
    let mut right_view: Vec<ratatui::text::Line<'_>> = Vec::new();

    // TODO: The contents of the views should probably be tables, especially when we choose to format HIR instructions
    // It might be nice to allow someone to navigate around and select or highlight HIR instructions
    // Do some fancy diff work to align things properly and highlight each side
    // BUG: Currently, some diffs are captured in a change block and we incorrectly count something as modified or updated when really there was
    // just one deletion. This should be cleaned up. This is because we consider any change to a line to be a change. Really we want something more granular.
    // For instance, if one argument to an instruction changes, we want to highlight that one argument -- not the entire instruction.
    // Maybe we need a second pass of diffing unfortunately? After we have bigger change blocks, we may want to realign and start to look at argument differences.
    // TODO: Highlight smaller segments for smaller changes. (For instance, if we only change an SSA variable, we should color that differently than the whole line)
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
    
    let outer = Block::bordered()
      .merge_borders(MergeStrategy::Exact)
      .title_bottom(
          Line::from(vec![
              Span::raw("[ "),
              Span::styled("←/→", Style::default().bold()),
              Span::raw(" Slide Analysis ] • [ "),
              Span::styled("⇧←/⇧→", Style::default().bold()),
              Span::raw(" Resize Analysis ] • [ "),
              Span::styled("↑/↓", Style::default().bold()),
              Span::raw(" Walk Block ] • [ "),
              Span::styled("⇥", Style::default().bold()),
              Span::raw(" Select Function ] • [ "),
              Span::styled("q", Style::default().bold()),
              Span::raw(" Quit ]"),
          ]).centered()
      );
    frame.render_widget(outer, main);
    
    let left_widget = Paragraph::new(left_view).block(Block::bordered()
        .title(left_title)
        .merge_borders(MergeStrategy::Exact));
    let right_widget = Paragraph::new(right_view).block(Block::new()
        .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
        .title(right_title)
        .merge_borders(MergeStrategy::Exact));

    frame.render_widget(left_widget, left_area);
    frame.render_widget(right_widget, right_area);

}
