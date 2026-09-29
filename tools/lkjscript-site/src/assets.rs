pub(crate) const CSS: &str = r#"
:root{color-scheme:light;--bg:#f8f9f5;--ink:#1d302b;--subtle:#5c6c65;--line:#d9e0d9;--accent:#126750;--paper:#fff;--code:#142e27}
*{box-sizing:border-box}
html{scroll-behavior:smooth}
body{margin:0;background:var(--bg);color:var(--ink);font:16px/1.65 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
a{color:var(--accent);text-decoration-thickness:1px;text-underline-offset:4px}
a:hover{color:#093f30}
a:focus-visible,input:focus-visible,button:focus-visible{outline:3px solid #ba7732;outline-offset:4px}
header,main,footer{max-width:1240px;margin:auto}
header{display:flex;justify-content:space-between;align-items:center;padding:27px 40px;border-bottom:1px solid var(--line);gap:24px}
.brand{font-size:25px;font-weight:800;color:var(--ink);text-decoration:none;letter-spacing:-1px;display:flex;align-items:center}
.brand>span:first-child{font-weight:450}
.badge{font-size:9px;letter-spacing:1.4px;font-weight:700;margin-left:16px;border:1px solid var(--line);border-radius:4px;padding:3px 6px}
nav{display:flex;gap:26px;flex-wrap:wrap}
nav a{text-decoration:none;font-size:14px;color:var(--ink)}
.hero{padding:88px 40px 72px;display:grid;grid-template-columns:1.35fr 1fr;gap:64px;align-items:center}
.eyebrow{letter-spacing:2px;font-size:11px;font-weight:750;color:var(--accent);margin:0 0 20px}
h1{font-size:clamp(42px,5.5vw,70px);letter-spacing:-3px;line-height:1.07;font-weight:650;margin:0 0 27px}
h1 em{font-style:normal;color:var(--accent)}
.lead{font-size:18px;max-width:560px;color:var(--subtle);margin:0 0 30px}
.actions{display:flex;gap:24px;align-items:center;flex-wrap:wrap}
.actions>a:not(.button){font-size:14px}
.button,button{background:var(--accent);color:#fff;border:0;border-radius:5px;padding:13px 18px;font:600 14px/1.5 system-ui,sans-serif;text-decoration:none;cursor:pointer}
.button:hover,button:hover{background:#0b4e3b;color:#fff}
.button span{margin-left:16px}
.note{font-size:12px;color:var(--subtle);margin-top:24px}
.terminal{background:var(--code);color:#eff8ef;border:1px solid #37564b;border-radius:12px;box-shadow:0 18px 46px #173c2113;transform:rotate(1deg);overflow:hidden}
.terminal-title{border-bottom:1px solid #37564b;font:600 10px/1.5 system-ui;letter-spacing:2px;padding:17px 24px;color:#b2cbbf}
.terminal pre{font-size:13px;padding:26px 24px 20px;margin:0;white-space:pre-wrap;overflow-wrap:anywhere;background:transparent}
.terminal code span{color:#9ebda9;font-size:10px;letter-spacing:1px}
.terminal-foot{padding:0 24px 24px;color:#a5c2b3;font-size:13px}
.principles{border-top:1px solid var(--line);border-bottom:1px solid var(--line);display:grid;grid-template-columns:repeat(3,1fr);padding:32px 40px;gap:40px}
.number{font-size:11px;letter-spacing:2px;color:var(--accent)}
.principles h2{font-size:18px;font-weight:650;margin:8px 0}
.principles p{font-size:14px;color:var(--subtle);margin:0}
.library{padding:64px 40px 80px}
.section-heading{display:flex;gap:40px;align-items:center;justify-content:space-between;margin-bottom:32px}
.section-heading h2{font-size:32px;line-height:1.25;letter-spacing:-1px;margin:0}
.section-heading .eyebrow{margin-bottom:12px}
.search{display:flex;gap:8px;max-width:430px;width:100%}
input{min-width:0;flex:1;font:14px/1.5 system-ui,sans-serif;background:var(--paper);border:1px solid var(--line);border-radius:5px;padding:12px 14px;color:var(--ink)}
.cards{display:grid;grid-template-columns:repeat(4,1fr);gap:14px}
.card{display:block;background:var(--paper);border:1px solid var(--line);border-radius:7px;text-decoration:none;padding:22px 20px}
.card:hover{border-color:var(--accent)}
.card h3{font-size:16px;margin:0 0 12px;color:var(--ink)}
.card h3 span{float:right;font-size:14px;color:var(--accent)}
.card p{font-size:13px;color:var(--subtle);margin:0}
footer{border-top:1px solid var(--line);padding:24px 40px 36px;display:flex;justify-content:space-between;gap:20px;flex-wrap:wrap;font-size:11px;color:var(--subtle)}
footer code{word-break:break-all}
.docs{display:grid;grid-template-columns:225px minmax(0,1fr);gap:64px;padding:42px 40px 80px}
.sidebar{align-self:start;position:sticky;top:28px}
.sidebar a{display:block;color:var(--subtle);font-size:13px;padding:9px 12px;text-decoration:none;border-left:2px solid transparent}
.sidebar a[aria-current]{border-color:var(--accent);color:var(--accent);background:#eaf0e9}
.sidebar .source-link{margin-top:25px;border-top:1px solid var(--line);padding-top:18px}
.document{max-width:820px}
.document-meta{display:flex;justify-content:space-between;gap:20px;font-size:10px;letter-spacing:1px;color:var(--subtle);margin-bottom:26px}
article{font-size:15px;overflow-wrap:anywhere}
article h1{font-size:38px;line-height:1.15;letter-spacing:-1.2px;margin:0 0 24px}
article h2{font-size:25px;letter-spacing:-.5px;margin:40px 0 14px;padding-top:12px;border-top:1px solid var(--line)}
article h3{font-size:19px;margin:30px 0 12px}
article h1,article h2,article h3,article h4{scroll-margin-top:24px}
article pre{background:#eef1ea;border:1px solid var(--line);border-radius:6px;padding:20px;overflow:auto;font-size:12px;line-height:1.65;white-space:pre}
code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}
article :not(pre)>code{background:#eaf0e7;border-radius:3px;padding:2px 4px;font-size:.88em}
article table{display:block;width:100%;overflow:auto;border-collapse:collapse;font-size:13px}
article th,article td{text-align:left;border:1px solid var(--line);padding:10px 12px;vertical-align:top}
article blockquote{margin-left:0;padding-left:20px;border-left:3px solid var(--accent);color:var(--subtle)}
.results{padding:60px 40px 90px;max-width:860px;margin:auto}
.results h1{font-size:42px;letter-spacing:-1px}
.results .search{max-width:none;margin-bottom:30px}
.results .card{margin-bottom:12px}
.sr-only,.skip:not(:focus){position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%)}
.skip:focus{display:block;padding:15px;background:#fff}
.empty{padding:26px 0;color:var(--subtle)}

@media(max-width:1000px){.hero{gap:30px}
.cards{grid-template-columns:repeat(2,1fr)}
.docs{gap:30px;grid-template-columns:185px minmax(0,1fr)}
}
@media(max-width:700px){header{padding:20px;align-items:flex-start;flex-direction:column;gap:16px}
nav{gap:22px}
.hero{padding:48px 20px;grid-template-columns:1fr;gap:38px}
h1{letter-spacing:-1.8px}
.terminal{transform:none}
.principles{grid-template-columns:1fr;padding:28px 20px;gap:25px}
.library{padding:42px 20px}
.section-heading{flex-direction:column;align-items:stretch;gap:24px}
.search{max-width:none}
.cards{grid-template-columns:1fr 1fr}
.card{padding:18px 14px}
.docs{display:block;padding:24px 20px 50px}
.sidebar{position:static;display:flex;flex-wrap:wrap;gap:4px;margin-bottom:32px}
.sidebar .eyebrow,.sidebar .source-link{display:none}
.sidebar a{font-size:12px;padding:6px 9px;border-left:0}
.sidebar a[aria-current]{border-radius:4px}
article h1{font-size:31px}
.results{padding:40px 20px}
footer{padding:24px 20px}
.badge{margin-left:12px}
}
@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}
}
@media print{header,.sidebar,.document-meta,footer,.search{display:none}
.docs{display:block;padding:0}
body{background:white;color:black}
a{color:inherit}
article pre{white-space:pre-wrap}
}

"#;
