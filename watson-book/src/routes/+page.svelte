<script>
	import Ipsum from './Ipsum.svelte';
	import Katex from './Katex.svelte';
</script>

<svelte:head>
	<link
		rel="stylesheet"
		href="https://cdn.jsdelivr.net/npm/katex@0.16.25/dist/katex.min.css"
		integrity="sha384-WcoG4HRXMzYzfCgiyfrySxx90XSl2rxY5mnVY5TwtWE6KLrArNKn0T/mOgNL0Mmi"
		crossorigin="anonymous"
	/>

	<!-- The loading of KaTeX is deferred to speed up page rendering -->
	<script
		defer
		src="https://cdn.jsdelivr.net/npm/katex@0.16.25/dist/katex.min.js"
		integrity="sha384-J+9dG2KMoiR9hqcFao0IBLwxt6zpcyN68IgwzsCSkbreXUjmNVRhPFTssqdSGjwQ"
		crossorigin="anonymous"
	></script>
</svelte:head>

<aside class="nav">
    <div class="section">I: Foundations</div>

    <ol class="chapter-list">
        <li>
            <div class="chapter"><span class="num">1</span> Logic</div>
            <ol class="part-list">
                <li class="part"><div><span class="num">1.1</span> Section the First</div></li>
                <li class="part selected"><div><span class="num">1.2</span> Section the Second</div></li>
                <li class="part"><div><span class="num">1.3</span> Section the Third</div></li>
            </ol>
        </li>
        <li>
            <div class="chapter"><span class="num">2</span> Set Theory</div>
            <ol class="part-list">
                <li class="part"><div><span class="num">2.1</span> Section the First</div></li>
                <li class="part"><div><span class="num">2.2</span> Section the Second</div></li>
                <li class="part"><div><span class="num">2.3</span> Section the Third</div></li>
            </ol>
        </li>
        <li>
            <div class="chapter"><span class="num">3</span> Numbers</div>
            <ol class="part-list">
                <li class="part"><div><span class="num">3.1</span> The Integers</div></li>
                <li class="part"><div><span class="num">3.2</span> The Rationals</div></li>
                <li class="part"><div><span class="num">3.3</span> The Reals</div></li>
            </ol>
        </li>
    </ol>

    <div class="section">II: Abstract Algebra</div>

    <ol class="chapter-list">
        <li>
            <div class="chapter"><span class="num">4</span> Groups</div>
            <ol class="part-list">
                <li class="part"><div><span class="num">4.1</span> Definitions</div></li>
                <li class="part"><div><span class="num">4.2</span> Subgroups</div></li>
                <li class="part"><div><span class="num">4.3</span> Homomorphisms and Isomorphisms</div></li>
            </ol>
        </li>
        <li>
            <div class="chapter"><span class="num">5</span> Rings</div>
            <ol class="part-list">
                <li class="part"><div><span class="num">5.1</span> Ring Structure</div></li>
                <li class="part"><div><span class="num">5.2</span> Ideals and Quotient Rings</div></li>
                <li class="part"><div><span class="num">5.3</span> Ring Homomorphisms</div></li>
            </ol>
        </li>
        <li>
            <div class="chapter"><span class="num">6</span> Fields</div>
            <ol class="part-list">
                <li class="part"><div><span class="num">6.1</span> Field Extensions</div></li>
                <li class="part"><div><span class="num">6.2</span> Algebraic and Transcendental Extensions</div></li>
                <li class="part"><div><span class="num">6.3</span> Finite Fields</div></li>
            </ol>
        </li>
    </ol>

</aside>

<div class="right">
</div>

<article>
	<h1>
		Logic
		<div class="chapter-num">1</div>
	</h1>

	<blockquote>
		One of the chief triumphs of modern mathematics consists in having discovered what mathematics
		really is.
	</blockquote>
	<p class="cite">&ndash; Bertrand Russell</p>

	<h2>Section the First <span class="num">1.1</span></h2>
	<p>
		Many people have learned from their highschool algebra classes that if we have an equation in
		the following form, a so called <em>quadratic equation</em>,
		<Katex>a x^2 + b x + c = 0</Katex>
		then the solutions of the equation will be
		<Katex displayMode>{'x = \\frac{-b \\pm \\sqrt{b^2 - 4 a c}}{2a}.'}</Katex>
	</p>

	<p>
		However what many people don't know is that we can add nonsense math expressions to our document
		to make it seem more sophisticated. Observe:
		<Katex displayMode
			>{'\\mathcal{L} \\left\\{ f(t) \\right\\} \\colonequals F(s) = \\int_{0}^{\\infty} e^{-st} f(t) \\, dt = \\sum_{n=0}^{\\infty} \\frac{(-1)^n}{n!} \\left( \\frac{d^n}{ds^n} G(s) \\right).'}
		</Katex>

		The derivation of this formula is, of course, trivial and so is left as an exercise to the
		reader.
	</p>

    <code>  
        <span class="line">1</span> <pre><span class="kw">theorem</span> <span class="name">or.intro_left</span> [p q : sentence] : (p)</pre> 
        <span class="line">2</span> <pre>    <span class="control">|-</span> p ∨ q</pre>
        <span class="line">3</span> <pre><span class="kw">proof</span> <span class="comment">-- this is ℝ</span></pre>
        <span class="line">4</span> <pre>    <span class="kw">have</span> <span class="control">assume</span> ¬p <span class="control">|-</span> q</pre>
        <span class="line">5</span> <pre>        <span class="kw">have</span> ⊥ <span class="kw">by</span> <span class="name">contradiction</span> [p];</pre>
        <span class="line">6</span> <pre>        <span class="kw">have</span> q <span class="kw">by</span> <span class="name">false.elim</span> [q];</pre>
        <span class="line">7</span> <pre>    ;</pre>
        <span class="line">8</span> <pre>    <span class="kw">have</span> (¬p) → q <span class="kw">by</span> imp.intro [¬p] [q];</pre>
        <span class="line">9</span> <pre><span class="kw">qed</span></pre>
    </code>

	<h3>We can talk about this <span class="num">1.1.1</span></h3>
	<Ipsum />

	<h3>Or we can talk about that <span class="num">1.1.2</span></h3>
	<Ipsum />

	<h2>Section the Second <span class="num">1.2</span></h2>
	<Ipsum />

	<h2>Section the Third <span class="num">1.3</span></h2>
	<Ipsum />
</article>

<link
	rel="stylesheet"
	type="text/css"
	href="https://cdn.jsdelivr.net/gh/dreampulse/computer-modern-web-font@master/fonts.css"
/>

<style>
    @import url('https://fonts.googleapis.com/css2?family=Barlow:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&family=Crimson+Pro:ital,wght@0,200..900;1,200..900&family=EB+Garamond:ital,wght@0,400..800;1,400..800&family=Fira+Code:wght@300..700&family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&family=Lora:ital,wght@0,400..700;1,400..700&display=swap');
    
    @font-face {
        font-family: "JuliaMono"; /* Assign a name to your font */
        src: url("https://cdnjs.cloudflare.com/ajax/libs/juliamono/0.051/JuliaMono-Regular.woff") format("woff2"); /* Specify the path and format */
        font-weight: normal; /* Define the font weight (e.g., normal, bold, 400, 700) */
        font-style: normal; /* Define the font style (e.g., normal, italic) */
        font-display: swap; /* Optional: Controls how font is displayed while loading */
    }
	    
    :global(*) {
		margin: 0;
		padding: 0;
	}

	:global(body) {
		background-color: #fefefe;
		color: #222;

        --nav-width: 260px;
	}

    :root {
        --blue: rgb(48, 112, 183);
    }

    .right {
        display: none;

        position: fixed;
        right: 0;
        top: 0;
        bottom: 0;
        width: calc(100vw - 128px - 128px - 700px - var(--nav-width));

        border-left: 1px solid lightgray;
        background: #f7f7f7;

    }

    .nav {
        position: fixed;
        top: 0;
        left: 0;
        bottom: 0;
        width: var(--nav-width);

        overflow: scroll;

        /* border-right: 1px solid lightgray; */
        padding: 8px;
        /* padding-right: 16px; */

		/* font-family: 'Inter', sans-serif; */
		font-family: 'Lora', serif;
		font-weight: 400;
		font-style: normal;
		font-size: 15px;
		/* line-height: 22px; */

        /* background: #f7f7f7; */
        /* background: rgb(27, 51, 78); */
        background: rgb(40, 51, 64);
        /* color: #ccc; */
        /* color: rgb(117, 162, 192); */
        /* color: rgb(129, 174, 204); */
        color: rgb(138, 173, 196);

        display: flex;
        flex-direction: column;
        justify-content: center;
    }

    .nav .section {
        text-transform: uppercase;
        /* font-weight: bold; */
		font-weight: 600;
        text-align: center;
        margin-top: 40px;
        margin-bottom: 8px;
        /* margin: 0; */
        /* margin-top: 16px; */

        /* color: #ddd; */

        /* display: grid; */
        /* grid-template-columns: 1fr auto 1fr; */
        /* gap: 8px; */
        /* align-items: center; */
        /* color: rgb(153, 191, 217); */
        /* color: rgb(153, 188, 212); */
        border-bottom: 2px solid rgb(48, 112, 183);
        border-bottom: 2px solid rgb(60, 106, 158);

        
    }

    .nav .chapter {
        /* font-style: italic; */
        /* color: #ddd; */
		font-weight: 600;
    }

    .nav .num {
        /* font-weight: bold; */
        /* color: #AAA; */
        color: rgb(98, 136, 161);
        color: rgb(103, 136, 159);
        font-weight: normal;
        font-variant-numeric: tabular-nums;
        font-style: normal;
    }


    .nav .section:first-child {
        margin-top: 16px;
    }
    /* .nav .section::before {
        content: '';
        display: inline-block;
        width: 100%;
        height: 2px;
        background: rgb(48, 112, 183);
    } */
    /* .nav .section::after {
        content: '';
        display: inline-block;
        width: 100%;
        height: 2px;
        background: rgb(48, 112, 183);
    } */

    .nav ol {
        list-style: none;
        margin: 0 12px;
    }

    .nav li {
        margin: 0;
        padding: 0;
        position: relative;
    }

    .nav li {
        margin-top: 6px;
    }

    .nav .chapter-list > li:first-child {
        /* margin-top: 12px; */
    }
    .nav .chapter-list > li:not(:first-child) {
        margin-top: 16px;
    }

    .nav .selected::after {
        display: block;
        content: '';
        
        position: absolute;
        top: -3px;
        bottom: -3px;
        left: -32px;
        /* height: 100%; */
        width: calc(var(--nav-width) + 16px);
        background: #70828b36;
        z-index: -1;
    }

    .chapter-num {
		display: flex;
		align-items: center;
		justify-content: center;

		position: absolute;
		right: 0;
		bottom: 0;

		background: rgb(48, 112, 183);
		border-radius: 2px;
		color: white;
		width: 80px;
		height: 80px;
		font-size: 40px;
	}


	article {
		font-family: 'Inter', sans-serif;
		/* font-family: 'Lora', serif; */
		font-weight: 400;
		font-style: normal;
		font-size: 16px;
		line-height: 25px;

		max-width: 700px;
		margin-left: calc(128px + var(--nav-width));
		margin-right: 128px;
		margin-bottom: 256px;
		margin-top: 112px;
	}

	blockquote {
		margin: 0 24px;
		font-style: italic;
		font-size: 18px;
	}

	blockquote::before {
		content: '“';
		position: absolute;
		margin-left: -0.4em;
    }

	blockquote::after {
		content: '”';
    }

	.cite {
		text-align: right;
	}

	h1,
	h2,
	h3 {
		font-family: 'Lora', serif;
        		/* font-family: 'Inter', sans-serif; */

		font-optical-sizing: auto;
		font-weight: 600;
		font-style: normal;

		position: relative;
		display: flex;
		justify-content: space-between;
		width: 100%;
	}
	h1 {
		font-size: 45px;
		margin-bottom: 32px;
		line-height: 45px;
	}

	h2 {
		font-size: 28px;
		margin-top: 64px;
		margin-bottom: 16px;
	}

	h2 .num {
		font-size: 24px;
	}

	h3 {
		font-size: 20px;
		margin-top: 24px;
		margin-bottom: 12px;
	}

	h3 .num {
		font-size: 18px;
	}

	:global(p + p),  :global(code + p) {
		margin-top: 16px;
	}

    :global(p + code),  :global(code + code) {
		margin-top: 8px;
	}

    code {
        font-family: "Fira Code", "JuliaMono", monospace;
        font-size: 15px;
        line-height: 22px;

        --num-gap: 24px;

        border-radius: 4px;
        margin-left: calc(-1ch - var(--num-gap));
        padding: 8px 0;

        display: grid;
        grid-template-columns: min-content auto;
        column-gap: var(--num-gap);
    }

    code pre {
        font-family: "Fira Code", "JuliaMono", monospace;
        font-size: 15px;
        line-height: 22px;
    }

    code .line {
        color: #7c7f93;
        user-select: none;
    }

    code .kw {
        color: #8839ef;
    }
    code .name {
        color: #1e66f5;
        font-style: italic;
    }
    code .control {
        color: #179299;
    }
    
    code .comment {
        color: #7c7f93;
        font-style: italic;
    }    

    

    :global(.katex) {
        color: #222 !important;
    }

    /* Makes katex fraction lines render a little thicker, which I think looks better */
	:global(.katex .mfrac .frac-line) {
		display: inline-block;
		width: 100%;
		border-bottom-width: 1px !important;
	}
</style>

