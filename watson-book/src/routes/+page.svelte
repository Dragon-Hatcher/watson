<script>
	import Ipsum from './Ipsum.svelte';
</script>

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

	<h2>Section the First <span class="num">1.1</span> </h2>
	<Ipsum />

	<h3>We can talk about this <span class="num">1.1.1</span> </h3>
	<Ipsum />

	<h3>Or we can talk about that <span class="num">1.1.2</span> </h3>
	<Ipsum />

	<h2>Section the Second <span class="num">1.2</span> </h2>
	<Ipsum />

	<h2>Section the Third <span class="num">1.3</span> </h2>
	<Ipsum />
</article>

<link
	rel="stylesheet"
	type="text/css"
	href="https://cdn.jsdelivr.net/gh/dreampulse/computer-modern-web-font@master/fonts.css"
/>

<style>
	@import url('https://fonts.googleapis.com/css2?family=Barlow:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&family=Crimson+Pro:ital,wght@0,200..900;1,200..900&family=EB+Garamond:ital,wght@0,400..800;1,400..800&family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&family=Lora:ital,wght@0,400..700;1,400..700&display=swap');

	:global(*) {
		margin: 0;
		padding: 0;
	}

	:global(body) {
		background-color: #fefefe;
		color: #222;
	}

	article {
		font-family: 'Inter', sans-serif;
		font-weight: 400;
		font-style: normal;
		font-size: 16px;
		line-height: 25px;

		max-width: 700px;
		margin-left: 128px;
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
		font-optical-sizing: auto;
		font-weight: 600;
		font-style: normal;

        position: relative;
        display: flex;
        justify-content: space-between;
        width: 100%;
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


	:global(p + p) {
		margin-top: 16px;
	}
</style>
