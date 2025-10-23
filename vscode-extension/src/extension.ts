import * as vscode from "vscode";

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

const DICTIONARY = [
  ["\\and", "∧"],
  ["\\or", "∨"],
  ["\\iff", "↔"],
  ["\\to", "→"],
  ["\\not", "¬"],
  ["\\bot", "⊥"],
  ["\\top", "⊤"],
];

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
    let prevPossibles = DICTIONARY.filter((d) => d[0].startsWith(text));

    if (prevPossibles.length > 0) {
      edit.replace(this.activeEditor!.document.uri, range, prevPossibles[0][1]);
      vscode.workspace.applyEdit(edit);
    }

    this.setActive(null);
  }

  updateSolution() {
    let newText = this.activeEditor!.document.getText(this.activeRange!);
    let possibles = DICTIONARY.filter((d) => d[0].startsWith(newText));

    if (possibles.length == 0) {
      this.commitSolution();
    } else {
      this.bestSolution = [newText, this.activeRange!];
    }
  }

  onChange(e: vscode.TextDocumentChangeEvent) {
    this.switchToActiveEditor();
    if (!this.activeEditor) return;
    if (this.activeEditor.document != e.document) return;

    for (let change of e.contentChanges) {
      if (this.activeRange == null && change.text == "\\") {
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
