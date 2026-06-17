import * as vscode from "vscode";
import { SHORTHAND_DICT } from "./shorthands";

export function activate(context: vscode.ExtensionContext) {
  console.log("Watson extension started.");

  const rewriter = new Rewriter();

  let disposable = vscode.workspace.onDidChangeTextDocument((e) =>
    rewriter.onChange(e)
  );
  context.subscriptions.push(disposable);

  vscode.window.onDidChangeTextEditorSelection((e) => rewriter.onMove(e));
}

// This method is called when your extension is deactivated
export function deactivate() {}

class Rewriter {
  activeEditor: vscode.TextEditor | null = null;
  activeRange: vscode.Range | null = null;
  bestSolution: [string, vscode.Range] | null = null;
  underliner: Underliner;

  constructor() {
    this.underliner = new Underliner();
  }

  switchToActiveEditor() {
    let editor = vscode.window.activeTextEditor;

    if (!editor) {
      this.activeEditor = null;
      this.setActive(null);
      return;
    }

    if (this.activeEditor == editor) return;

    // We have a new document. Reset our state.
    this.setActive(null);

    if (editor.document.languageId == "watson") {
      this.activeEditor = editor;
    }
  }

  setActive(r: vscode.Range | null) {
    if (r == null) {
      this.bestSolution = null;
    }

    this.activeRange = r;
    this.underliner.setRange(this.activeEditor, r);
  }

  commitSolution() {
    if (!this.bestSolution) return;

    let edit = new vscode.WorkspaceEdit();
    let [text, range] = this.bestSolution;
    let prevPossibles = SHORTHAND_DICT.filter((d) => d[0].startsWith(text));

    if (prevPossibles.length > 0) {
      edit.replace(this.activeEditor!.document.uri, range, prevPossibles[0][1]);
      vscode.workspace.applyEdit(edit);
    }

    this.setActive(null);
  }

  updateSolution() {
    let newText = this.activeEditor!.document.getText(this.activeRange!);

    if (newText == "\\") {
      this.bestSolution = null;
      return;
    }

    // Grouped sub/superscripts: \_{...} or \^{...}. While the group is being
    // typed we only keep the active range alive. The conversion itself happens
    // in onMove, because VSCode auto-closes "{" into "{}" with the cursor in
    // the middle, so the closing brace is present from the start and only the
    // cursor position tells us when the user is actually done.
    if (/^\\[_^]\{[^}]*\}?$/.test(newText)) {
      this.bestSolution = null;
      return;
    }

    let possibles = SHORTHAND_DICT.filter((d) => d[0].startsWith(newText));

    if (possibles.length == 1 && possibles[0][0] == newText) {
      this.bestSolution = [newText, this.activeRange!];
      this.commitSolution();
    } else if (possibles.length == 0) {
      this.commitSolution();
    } else {
      this.bestSolution = [newText, this.activeRange!];
    }
  }

  // Converts each character in `inner` to its sub- ("_") or super- ("^")
  // script form using the existing shorthand entries. Characters without a
  // mapping (e.g. there is no subscript "y") are kept as-is.
  convertGroup(kind: string, inner: string): string {
    let result = "";
    for (let c of inner) {
      let entry = SHORTHAND_DICT.find((d) => d[0] === "\\" + kind + c);
      result += entry ? entry[1] : c;
    }
    return result;
  }

  replaceRange(range: vscode.Range, replacement: string) {
    let edit = new vscode.WorkspaceEdit();
    edit.replace(this.activeEditor!.document.uri, range, replacement);
    vscode.workspace.applyEdit(edit);
    this.setActive(null);
  }

  isInMathMode(position: vscode.Position): boolean {
    const line = this.activeEditor!.document.lineAt(position.line);
    const textBeforeCursor = line.text.substring(0, position.character);

    // Count unmatched $ signs
    let dollarCount = 0;
    let i = 0;
    while (i < textBeforeCursor.length) {
      if (textBeforeCursor[i] === '$') {
        // Check if it's $$
        if (i + 1 < textBeforeCursor.length && textBeforeCursor[i + 1] === '$') {
          dollarCount++;
          i += 2;
        } else {
          dollarCount++;
          i++;
        }
      } else {
        i++;
      }
    }

    // If there's an odd number of dollar signs, we're inside math mode
    return dollarCount % 2 === 1;
  }

  onChange(e: vscode.TextDocumentChangeEvent) {
    this.switchToActiveEditor();
    if (!this.activeEditor) return;
    if (this.activeEditor.document != e.document) return;

    for (let change of e.contentChanges) {
      if (this.activeRange == null && change.text == "\\") {
        // Don't start replacement if we're in math mode
        if (this.isInMathMode(change.range.start)) {
          continue;
        }

        // Start a new active range:
        let range = new vscode.Range(
          change.range.start,
          change.range.start.translate(0, 1)
        );
        this.setActive(range);
      } else if (
        this.activeRange != null &&
        this.activeRange.contains(change.range)
      ) {
        let lengthChange = change.text.length - change.rangeLength;
        let newEnd = this.activeRange.end.translate(0, lengthChange);
        let newRange = this.activeRange.with({ end: newEnd });
        this.setActive(newRange);
        this.updateSolution();
      } else {
        this.setActive(null);
      }
    }
  }

  onMove(e: vscode.TextEditorSelectionChangeEvent) {
    if (!this.activeRange) return;

    this.switchToActiveEditor();
    if (!this.activeEditor) return;
    if (this.activeEditor != e.textEditor) return;

    // Finish a \_{...} / \^{...} group once the cursor moves onto or past the
    // closing brace (by typing or overtyping "}"). VSCode auto-closes "{" to
    // "{}", so the brace exists from the start and only the cursor position
    // tells us the group is complete.
    let text = this.activeEditor.document.getText(this.activeRange);
    let group = text.match(/^\\([_^])\{([^}]*)\}$/);
    if (group) {
      let cursor = e.selections[0].active;
      if (cursor.isAfterOrEqual(this.activeRange.end)) {
        this.replaceRange(
          this.activeRange,
          this.convertGroup(group[1], group[2])
        );
      } else if (!this.activeRange.contains(cursor)) {
        // Cursor left the braces without closing the group; abandon it.
        this.setActive(null);
      }
      return;
    }

    if (e.selections.some((s) => !this.activeRange?.contains(s.anchor))) {
      this.commitSolution();
      this.setActive(null);
    }
  }
}

class Underliner {
  editor: vscode.TextEditor | null = null;
  deco: vscode.TextEditorDecorationType;

  constructor() {
    this.deco = vscode.window.createTextEditorDecorationType({
      textDecoration: "underline",
    });
  }

  setRange(editor: vscode.TextEditor | null, range: vscode.Range | null) {
    if (range == null) {
      this.editor?.setDecorations(this.deco, []);
      this.editor = null;
    } else {
      this.editor = editor;
      this.editor?.setDecorations(this.deco, [range]);
    }
  }
}
