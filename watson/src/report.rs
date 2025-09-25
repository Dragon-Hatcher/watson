use crate::context::Ctx;
use crate::util::ansi::{ANSI_BOLD, ANSI_GREEN, ANSI_RED, ANSI_RESET, ANSI_YELLOW};
use crate::util::plural;

pub fn display_report(ctx: &Ctx) -> bool {
    let statuses = &ctx.proof_statuses;

    println!(
        "Checked {} theorem{} ({} axiom{}, {} theorem{}):",
        statuses.total_cnt(),
        plural(statuses.total_cnt()),
        statuses.axiom_cnt(),
        plural(statuses.axiom_cnt()),
        statuses.theorem_cnt(),
        plural(statuses.theorem_cnt())
    );

    println!(
        " {ANSI_GREEN}✓{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} theorem{} correct. ",
        statuses.correct_cnt(),
        plural(statuses.correct_cnt())
    );
    if statuses.todo_cnt() > 0 {
        println!(
            " {ANSI_YELLOW}~{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} theorem{} with todo.",
            statuses.todo_cnt(),
            plural(statuses.todo_cnt())
        );
    }
    if statuses.error_cnt() > 0 {
        println!(
            " {ANSI_RED}✗{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} theorem{} with errors.",
            statuses.error_cnt(),
            plural(statuses.error_cnt())
        );
    }

    if !statuses.circular_dependencies().is_empty() {
        println!(
            " {ANSI_RED}✗{ANSI_RESET} {ANSI_BOLD}{}{ANSI_RESET} circular dependency group{} detected.",
            statuses.circular_dependencies().len(),
            plural(statuses.circular_dependencies().len())
        );

        for group in statuses.circular_dependencies() {
            print!("     -");
            for (i, thm) in group.iter().enumerate() {
                if i > 0 {
                    print!(",");
                }
                print!(" {}", thm.name());
            }
            println!();
        }
    }

    let all_ok = statuses.error_cnt() == 0 && statuses.circular_dependencies().is_empty();

    if all_ok {
        println!();
        println!("🎉 All theorems correct! 🎉");
    }

    all_ok
}
