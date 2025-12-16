declare function log(...: any): number

declare class Span
end

declare class UnResFrag
    span: Span
end

declare class UnResAnyFrag
    span: Span
end

declare class UnResFact
    assumption: UnResFrag?
    conclusion: UnResFrag
end

declare class Frag
    formal: Frag
end

declare class Fact
    assumption: Frag?
    conclusion: Frag
end

declare class Scope
end

declare class Theorem
    name: string
    hypotheses: {Fact}
    conclusion: Frag
end

declare class ProofState
    theorem: Theorem
    function addAssumption(self, assumption: Frag): ProofState
    function popAssumption(self, justifying: Frag): ProofState
    function applyTheorem(self, thm: Theorem, templates: {Frag}): ProofState
    function applyTodo(self, justifying: Frag): ProofState
    function applyError(self, justifying: Frag): ProofState
end

declare class Diagnostic
    function withError(self, msg: string, span: Span): Diagnostic
    function withInfo(self, msg: string, span: Span): Diagnostic
end

declare class DiagnosticMeta
    function new(self, title: string): Diagnostic
end

declare Diagnostic: DiagnosticMeta