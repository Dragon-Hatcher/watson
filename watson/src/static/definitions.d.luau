declare function log(...: any): number

declare class UnResFrag
end

declare class UnResAnyFrag
end

declare class UnResFact
    assumption: UnResFrag?
    conclusion: UnResFrag
end

declare class Frag
    formal: Frag
end

declare class Fact
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
