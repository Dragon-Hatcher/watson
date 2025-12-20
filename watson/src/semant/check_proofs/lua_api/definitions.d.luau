--- Prints formatted values to stderr for debugging tactics.
--- Multiple arguments are separated by spaces. Non-string values are pretty-printed.
--- @param ... Values to log
declare function log(...: any)

--- Represents a source location range (span) in the input text.
--- Used to attach source position information to fragments and diagnostics.
declare class Span
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
end

declare FormalCat: FormalCatMeta

--- A fragment - a piece of syntax in a formal category represented as a tree of
--- the notation used to define it 
declare class Frag
    --- The formal syntax category this fragment belongs to
    cat: FormalCat
    --- The formal (canonical) representation of this fragment reduced to only
    --- syntax from the formal language with no user defined notation
    formal: Frag

    --- Converts this fragment to a fact with no assumption (a standalone statement).
    --- @return A fact with this fragment as the conclusion
    function fact(self): Fact

    --- Instantiates template holes in this fragment with concrete fragments.
    --- @param templates Array of fragments to substitute for template holes 
    --- @return A new fragment with templates substituted
    function instantiateTemplates(self, templates: {Frag}): Frag

    --- Attempt to match this fragment to the provided pattern. If the pattern
    --- contains holes the corresponding subfragments of this fragment are 
    --- returned as a map.
    --- @param pattern the pattern to match
    --- @return a map from hole indexes to the matched fragments or nil if no match
    function match(self, pattern: Frag): {[number]: Frag}?
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

    --- Change the goal of this TacticInfo.
    --- @param f the new goal
    --- @return a new TacticInfo containing the same reasoning chain with the new goal
    function withGoal(self, f: Frag): TacticInfo
end

--- A diagnostic (error or warning) with source annotations.
--- Diagnostics are built up by adding error/info annotations, then reported.
declare class Diagnostic
    --- Adds an error annotation at the given source location.
    --- @param msg The error message
    --- @param span The source location to highlight
    --- @return A new diagnostic with the annotation added
    function withError(self, msg: string, span: Span): Diagnostic

    --- Adds an informational annotation at the given source location.
    --- @param msg The informational message
    --- @param span The source location to highlight
    --- @return A new diagnostic with the annotation added
    function withInfo(self, msg: string, span: Span): Diagnostic

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
    --- @param title The main error message
    --- @return A new diagnostic
    function new(self, title: string): Diagnostic
end

declare Diagnostic: DiagnosticMeta
