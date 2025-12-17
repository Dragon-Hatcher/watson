declare function log(...: any): number

declare class Span
end

declare class SpannedString
    span: Span
    str: string
end

declare class UnResFrag
    span: Span
    function resolve(self, scope: Scope): (Frag?, Diagnostic?)
end

declare class UnResAnyFrag
    span: Span
    function resolve(self, scope: Scope, cat: FormalCat): (Fact?, Diagnostic?) --
end

declare class UnResFact
    assumption: UnResFrag?
    conclusion: UnResFrag
    function resolve(self, scope: Scope): (Fact?, Diagnostic?, Diagnostic?)
end

declare class FormalCat
    name: string
end

declare class Frag
    cat: FormalCat
    formal: Frag
end

declare FragMap : {
    new: <S>(self: any) -> FragMap<S>
}

export type FragMap<T> = {
    set: (any, f: Frag, val: T) -> T?,
    get: (any, f: Frag) -> T?
}

declare class Fact
    assumption: Frag?
    conclusion: Frag
    formal: Fact
end

declare class FactMeta
    function new(self, assumption: Frag?, conclusion: Frag): Fact
    function newC(self, conclusion: Frag): Fact
end

declare Fact: FactMeta

declare FactMap : {
    new: <S>(self: any) -> FactMap<S>
}

export type FactMap<T> = {
    set: (any, f: Fact, val: T) -> T?,
    get: (any, f: Fact) -> T?
}

declare class Binding
end

declare class Scope
    function bindFrag(self, binding: Binding, frag: Frag): Scope
    function bindHole(self, binding: Binding, cat: FormalCat, idx: number): Scope
end

declare class Template
    cat: FormalCat
    binding: Binding
end

declare class Theorem
    name: string
    templates: {Template}
    hypotheses: {Fact}
    conclusion: Frag
    scope: Scope
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
    function report(self)
end

declare class DiagnosticMeta
    function new(self, title: string): Diagnostic
end

declare Diagnostic: DiagnosticMeta
