--- Prints formatted values to stderr for debugging tactics.
--- Multiple arguments are separated by spaces. Non-string values are pretty-printed.
--- @param ... Values to log
declare function log(...: any)

--- Represents a source location range (span) in the input text.
--- Used to attach source position information to fragments and diagnostics.
declare class Span
    --- Create an error DiagnosticSpan with the given message.
    --- @param msg The message to attach to the span
    --- @return The new DiagnosticSpan
    function label(self, msg: string): DiagnosticSpan

    --- Create a warning DiagnosticSpan with the given message.
    --- @param msg The message to attach to the span
    --- @return The new DiagnosticSpan
    function labelWarning(self, msg: string): DiagnosticSpan

    --- Create an info DiagnosticSpan with the given message.
    --- @param msg The message to attach to the span
    --- @return The new DiagnosticSpan
    function labelInfo(self, msg: string): DiagnosticSpan
end

--- A string value paired with its source location.
--- Used to preserve source position information for names and literals.
declare class SpannedString
    --- The source location of this string
    span: Span
    --- The string value
    str: string
end

--- An unresolved fragment - a parse tree that can be resolved to a Frag.
--- The category is inferred from context during resolution.
declare class UnResFrag
    --- The source location of this fragment
    span: Span

    --- Resolves this parse tree to a fragment by looking up names in the scope.
    --- @param scope The scope to use for name resolution
    --- @return frag The resolved fragment, or nil if resolution failed
    --- @return diag A diagnostic explaining the resolution failure, or nil if successful
    function resolve(self, scope: Scope): (Frag?, Diagnostic?)
end

declare class UnResFragMeta
    --- Attempt to parse the given string as a fragment of the given category
    --- @param text the string to parse
    --- @param cat the formal category to parse the string as
    --- @return an unresolved fragment if the parse was successful or a diagnostic
    --- why it was not 
    function parse(self, text: string, cat: FormalCat): (UnResFrag?, Diagnostic?)
end

declare UnResFrag: UnResFragMeta

--- An unresolved fragment with an unknown resolution category.
--- Unlike UnResFrag, this requires an explicit category to be provided.
declare class UnResAnyFrag
    --- The source location of this fragment
    span: Span

    --- Resolves this parse tree to a fragment of the specified category.
    --- @param scope The scope to use for name resolution
    --- @param cat The formal syntax category to parse this fragment as
    --- @return frag The resolved fragment, or nil if resolution failed
    --- @return diag A diagnostic explaining the resolution failure, or nil if successful
    function resolve(self, scope: Scope, cat: FormalCat): (Frag?, Diagnostic?)
end

--- An unresolved fact with an optional assumption and a conclusion.
--- Both parts are unresolved fragments that need to be resolved separately.
declare class UnResFact
    --- The assumption part (if any) - represents the "if" part of an implication
    assumption: UnResFrag?
    --- The conclusion part - represents what is being proven
    conclusion: UnResFrag

    --- Resolves both the assumption and conclusion to produce a Fact.
    --- @param scope The scope to use for name resolution
    --- @return fact The resolved fact, or nil if either part failed to resolve
    --- @return assumption_diag A diagnostic for assumption resolution failure, or nil
    --- @return conclusion_diag A diagnostic for conclusion resolution failure, or nil
    function resolve(self, scope: Scope): (Fact?, Diagnostic?, Diagnostic?)
end

--- A formal syntax category. These correspond to syntax_category definitions in
--- Watson source files.
declare class FormalCat
    --- The name of this category.
    name: string
end

declare class FormalCatMeta
    --- Get the formal category of sentences
    --- @return the formal cat for sentences
    function sentence(self): FormalCat

    --- Get the formal category with the given name
    --- @param name the name of the formal cat
    --- @return the formal cat with that name or nil if it doesn't exist
    function get(self, name: string): FormalCat?
end

declare FormalCat: FormalCatMeta

--- A fragment - a piece of syntax in a formal category represented as a tree of
--- the notation used to define it 
declare class Frag
    --- The formal syntax category this fragment belongs to
    cat: FormalCat

    --- Children of this fragment
    children: {Frag}

    --- If this fragment is notation for a different fragment, this is the node
    --- in which the children will be replaced to get that fragment.
    replacement: Frag?

    --- If this fragment is notation for a different fragment, what was the
    --- source of that definition.
    source: DefinitionSource?

    --- If this fragment is a variable, the index of that variable
    varIdx: number?

    --- If this fragment is a template, the index of that template
    templateIdx: number?

    --- If this fragment is a hole, the index of that hole
    holeIdx: number?

    --- The number of unclosed (aka free) variables in the fragment
    unclosedCount: number

    --- The formal (canonical) representation of this fragment reduced to only
    --- syntax from the formal language with no user defined notation
    formal: Frag

    --- Converts this fragment to a fact with no assumption (a standalone statement).
    --- @return A fact with this fragment as the conclusion
    function fact(self): Fact

    --- Returns a new fragment with the same formal meaning but whose notation
    --- is just the single given name.
    --- @param name the name for the notation
    --- @return the new fragment
    function named(self, name: string): Frag

    --- This same fragment but with the names changed in the bindings at the
    --- top layer. 
    --- @param names the new binding names
    --- @return the new fragment
    function changeBinderNames(self, names: {string}): Frag

    --- Instantiates holes in this fragment with concrete fragments.
    --- @param templates Array of fragments to substitute for the holes 
    --- @return A new fragment with the holes substituted
    function instantiateHoles(self, holes: {Frag}): Frag

    --- Instantiates template holes in this fragment with concrete fragments.
    --- @param templates Array of fragments to substitute for template holes 
    --- @return A new fragment with templates substituted
    function instantiateTemplates(self, templates: {Frag}): Frag

    --- Instantiates variables in this fragment with concrete fragments.
    --- When an n argument array is passed the first n variables are substituted
    --- @param templates Array of fragments to substitute for variables 
    --- @return A new fragment with variables substituted
    function instantiateVars(self, templates: {Frag}): Frag

    --- Create a new frag where the top level notation has been changed to the
    --- notation that was used in the definition.
    --- @return the new reduced fragment
    function reduce(self): Frag

    --- Attempt to match this fragment to the provided pattern. If the pattern
    --- contains holes the corresponding subfragments of this fragment are 
    --- returned as a map.
    --- @param pattern the pattern to match
    --- @return a map from hole indexes to the matched fragments or nil if no match
    function match(self, pattern: Frag): {[number]: Frag}?
end

declare class FragMeta
    --- Construct a variable with the given category and index
    --- @param the formal category of the variable
    --- @param the debruijn index of the variable
    --- @return the fragment for the variable 
    function var(self, cat: FormalCat, idx: number): Frag

    --- Construct a hole with the given category and index
    --- @param the formal category of the holes
    --- @param the index of the holes
    --- @return the fragment for the hole 
    function hole(self, cat: FormalCat, idx: number): Frag
end

--- Metatable for constructing frags.
declare Frag: FragMeta


declare class DefinitionSource
    isDefinitionCmd: CommandId?
    isSyntaxCmd: CommandId?
    isTemplate: boolean
    isDefinitionHole: boolean
    isBinding: boolean
    isLuaApi: boolean
end

declare class CommandId
end

--- Metatable for creating fragment maps.
declare FragMap : {
    new: <S>(self: any) -> FragMap<S>
}

--- A map from Fragments to any data type. Keys use structural not referential
--- equality. This is backed by an immutable map so copies may share values so
--- you need to be careful if using mutable values in the map.
export type FragMap<T> = {
    --- Associates a fragment with a value.
    --- @param f The fragment key
    --- @param val The value to store
    --- @return The previous value for this key, or nil
    set: (any, f: Frag, val: T) -> T?,

    --- Retrieves the value associated with a fragment.
    --- @param f The fragment key
    --- @return The value, or nil if not found
    get: (any, f: Frag) -> T?,

    --- Creates a shallow copy of this map.
    --- @return A new map with the same contents
    copy: <S>(self: S) -> S,
}

--- A logical fact with an optional assumption and a conclusion.
--- Represents statements like "A |- B" (if A then B) or just "|- B" (B holds).
declare class Fact
    --- The assumption/hypothesis (the "if" part), or nil for unconditional facts
    assumption: Frag?
    --- The conclusion (the "then" part, or just what is proven if no assumption)
    conclusion: Frag
    --- The formal (canonical) representation of this fact
    formal: Fact

    --- Instantiates template holes in both assumption and conclusion.
    --- @param templates Array of fragments to substitute for template holes
    --- @return A new fact with templates substituted
    function instantiateTemplates(self, templates: {Frag}): Fact
end

declare class FactMeta
    --- Creates a new fact with an assumption and conclusion.
    --- @param assumption The hypothesis, or nil for an unconditional fact
    --- @param conclusion The conclusion being proven
    --- @return A new fact
    function new(self, assumption: Frag?, conclusion: Frag): Fact

    --- Creates a new fact with only a conclusion (no assumption).
    --- @param conclusion The conclusion being proven
    --- @return A new unconditional fact
    function newC(self, conclusion: Frag): Fact
end

--- Metatable for constructing facts.
declare Fact: FactMeta

--- Metatable for creating fact maps.
declare FactMap : {
    new: <S>(self: any) -> FactMap<S>
}

--- A map from Fragments to any data type. Keys use structural not referential
--- equality. This is backed by an immutable map so copies may share values so
--- you need to be careful if using mutable values in the map.
export type FactMap<T> = {
    --- Associates a fact with a value.
    --- @param f The fact key
    --- @param val The value to store
    --- @return The previous value for this key, or nil
    set: (any, f: Fact, val: T) -> T?,

    --- Retrieves the value associated with a fact.
    --- @param f The fact key
    --- @return The value, or nil if not found
    get: (any, f: Fact) -> T?,

    --- Creates a shallow copy of this map.
    --- @return A new map with the same contents
    copy: <S>(self: S) -> S,
}

--- A notation binding - represents a pattern that can be bound in a scope.
--- Bindings are created from notation patterns and associate with scope entries.
declare class Binding
    --- The formal syntax category this binding binds
    cat: FormalCat

    --- Any names within the binding
    names: {string}
end

declare class BindingMeta
    --- Get the binding for a single name in the given category
    --- @param name the name to attach the binding too
    --- @param cat the formal category of the binding
    --- @return the binding for the name
    function name(self, name: string, cat: FormalCat): Binding
end

declare Binding: BindingMeta

--- A scope for name resolution.
--- Scopes are immutable - binding operations return new child scopes.
declare class Scope
    --- Creates a child scope with a fragment bound to a notation pattern.
    --- @param binding The notation pattern to bind
    --- @param frag The fragment to bind to this pattern
    --- @return A new scope extending this one with the binding
    function bindFrag(self, binding: Binding, frag: Frag): Scope

    --- Creates a child scope with a hole bound to a notation pattern.
    --- @param binding The notation pattern to bind
    --- @param idx The index of the hole (0-based)
    --- @return A new scope extending this one with the hole binding
    function bindHole(self, binding: Binding, idx: number): Scope
end

declare class ScopeMeta
    --- Get the active scope when the tactic with the given name was declared.
    --- @param name the name of the tactic
    --- @return the scope for that tactic or none if no tactic with that name exists
    function atTactic(self, name: string): Scope?

    --- Get the active scope at the end of parsing.
    function atEnd(self): Scope
end

declare Scope: ScopeMeta

--- A template parameter for a theorem.
--- Templates are instantiated when applying the theorem in a proof.
declare class ThmTemplate
    --- The formal syntax category for this template parameter
    cat: FormalCat
    --- The notation binding for this template in the theorem's scope
    binding: Binding
    --- Bindings for holes within this template
    holes: {Binding}
end

--- A theorem - a proven statement that can be applied in proofs.
declare class Theorem
    --- The name of this theorem
    name: string
    --- Template parameters that must be instantiated when applying this theorem
    templates: {ThmTemplate}
    --- Hypotheses that must be satisfied to apply this theorem
    hypotheses: {Fact}
    --- The conclusion that is proven when this theorem is applied
    conclusion: Frag
    --- The scope that the theorem was created with. Also binds all the 
    --- templates of the theorem.
    scope: Scope
end

declare class TheoremMeta
    --- Looks up a theorem by name.
    --- @param name The theorem name
    --- @return The theorem, or nil if not found
    function lookupByName(self, name: string): Theorem?
end

declare Theorem: TheoremMeta

--- The current state during proof checking.
--- Proof operations return a new state (states are immutable).
declare class ProofState
    --- The theorem being proven
    theorem: Theorem

    --- Adds an assumption to the current goal.
    --- This introduces a hypothesis that can be used later in the proof.
    --- @param assumption The fragment to assume
    --- @return A new proof state with the assumption added
    function addAssumption(self, assumption: Frag): ProofState

    --- Pops (discharges) an assumption from the current goal.
    --- This proves a fact by proving the conclusion under an assumption.
    --- @param justifying The fragment that the assumption justified
    --- @return A new proof state with the assumption removed and the fact added
    function popAssumption(self, justifying: Frag): ProofState

    --- Applies a theorem by adding its conclusion to the list of known facts.
    --- The hypotheses must already have been previously proven.
    --- @param thm The theorem to apply
    --- @param templates Fragments to instantiate the theorem's template parameters
    --- @return A new proof state after applying the theorem
    function applyTheorem(self, thm: Theorem, templates: {Frag}): ProofState

    --- Add a frag to the list of known facts while marking the proof as TODO.
    --- Used for incomplete proofs during development.
    --- @param justifying The fragment representing the fact being proved
    --- @return A new proof state with the fact
    function applyTodo(self, justifying: Frag): ProofState

    --- Add a frag to the list of known facts while marking the proof as errored.
    --- Used when tactics fail to allow latter tactics to continue,
    --- @param justifying The fragment representing the fact being proved
    --- @return A new proof state with the fact
    function applyError(self, justifying: Frag): ProofState
end

--- A representation of the reasoning chain of the tactic which can be
--- presented to the user.
declare class TacticInfo
    --- Add an assumption to the reasoning chain.
    --- @param f the assumption to add
    --- @return a new TacticInfo containing the assumption
    function withAssume(self, f: Frag): TacticInfo

    --- Add a deduction to the reasoning chain.
    --- @param f the fact that was deduced
    --- @return a new TacticInfo containing the deduction
    function withDeduce(self, f: Fact): TacticInfo

    --- Add a let binding to the reasoning chain
    --- @param binding the binding being added
    --- @param replacement what is bound to that binding or nil if we want it hidden
    --- @return a new TacticInfo containing the let binding
    function withLet(self, binding: Binding, replacement: Frag?): TacticInfo

    --- Change the goal of this TacticInfo.
    --- @param f the new goal
    --- @return a new TacticInfo containing the same reasoning chain with the new goal
    function withGoal(self, f: Frag): TacticInfo
end

declare class AttributeTracker
    --- Get the list of attributes for the given command.
    --- @param the command to get the attributes for
    --- @return a list of the attributes
    function getAttributes(self, cmd: CommandId): {Attribute}
end

declare class AttributeTrackerMeta
    --- Get the attribute state at the end of parsing.
    function atEnd(self): AttributeTracker
end

declare AttributeTracker : AttributeTrackerMeta

--- A span of text with an attached message.
declare class DiagnosticSpan
end

--- A diagnostic (error or warning) with source annotations.
--- Diagnostics are built up by adding error/info annotations, then reported.
declare class Diagnostic
    --- Adds an error annotation at the given source location.
    --- @param msg The error message
    --- @param spans List of source locations to highlight
    --- @return A new diagnostic with the annotation added
    function withError(self, msg: string, ...: DiagnosticSpan): Diagnostic

    --- Adds an informational annotation at the given source location.
    --- @param msg The informational message
    --- @param spans List of source locations to highlight
    --- @return A new diagnostic with the annotation added
    function withInfo(self, msg: string, ...: DiagnosticSpan): Diagnostic

    --- Adds the reasoning chain given by the TacticInfo.
    --- @param info The TacticInfo to add
    --- @return A new diagnostic with the info added
    function withTacticInfo(self, info: TacticInfo): Diagnostic

    --- Reports this diagnostic (adds it to the diagnostic list for the current theorem).
    --- After reporting, the diagnostic will be displayed to the user.
    function report(self)
end

declare class DiagnosticMeta
    --- Creates a new diagnostic with the given title.
    --- @param msg The main error message
    --- @param spans List of source locations to highlight
    --- @return A new diagnostic
    function new(self, msg: string, ...: DiagnosticSpan): Diagnostic

    --- Creates a new warning diagnostic with the given title.
    --- @param msg The main warning message
    --- @param spans List of source locations to highlight
    --- @return A new diagnostic
    function newWarning(self, msg: string, ...: DiagnosticSpan): Diagnostic
end

declare Diagnostic: DiagnosticMeta



declare class VFunction
    function with(self, args: {VTerm}): VTerm
end

declare class VFunctionMeta
    function new(self, name: string, arity: number): VFunction
end

declare VFunction: VFunctionMeta


declare class VPredicate
    function with(self, args: {VTerm}): VFormula
end

declare class VPredicateMeta
    function new(self, name: string, arity: number): VPredicate
end

declare VPredicate: VPredicateMeta


declare class VTerm
end

declare class VTermMeta
    function newVar(self, idx: number): VTerm
end

declare VTerm: VTermMeta


declare class VFormula
end

declare class VFormulaMeta
    function newEq(self, t1: VTerm, t2: VTerm): VFormula
    function newAnd(self, ...: VFormula): VFormula
    function newOr(self, ...: VFormula): VFormula
    function newNot(self, f: VFormula): VFormula
    function newImp(self, ant: VFormula, cons: VFormula): VFormula
    function newIff(self, f1: VFormula, f2: VFormula): VFormula
    function newForall(self, var: number, f: VFormula): VFormula
    function newExists(self, var: number, f: VFormula): VFormula
    function newTrue(self): VFormula
    function newFalse(self): VFormula
end

declare VFormula: VFormulaMeta


declare class VOptions
    function setTimeout(self, timeout_ms: number)
end

declare class VOptionsMeta
    function new(self): VOptions
end

declare VOptions: VOptionsMeta


declare class VProblem
    function addAxiom(self, axiom: VFormula)
    function setConjecture(self, conj: VFormula)
    function solve(self): (VProof?, "proved" | "unprovable" | "unknown")
end

declare class VProblemMeta
    function new(self, options: VOptions): VProblem
end

declare VProblem: VProblemMeta 

declare class VProof
    stepCount: number
    function getStep(self, idx: number): VProofStep
end

declare class VProofStep
    discoveryOrder: number
    conclusion: VFormula
    rule: string
    premises: {number}
end