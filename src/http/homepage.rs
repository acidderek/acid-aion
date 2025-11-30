pub fn homepage_html(
    health_score: f32,
    health_label: &str,
    awareness_score: f32,
    awareness_label: &str,
) -> String {
    // All non-placeholder braces are doubled because `format!` uses `{}`.
    format!(
r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>AION Kernel</title>
  <style>
    body {{
      font-family: system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
      background:#060712;
      color:#f5f5f5;
      margin:0;
      padding:2rem;
    }}
    .card {{
      max-width: 640px;
      margin: 0 auto;
      background: #10121f;
      border-radius: 12px;
      padding: 1.5rem 2rem;
      box-shadow: 0 18px 45px rgba(0,0,0,.4);
    }}
    .badge {{
      display:inline-block;
      padding:0.15rem 0.6rem;
      border-radius:999px;
      font-size:0.75rem;
      text-transform:uppercase;
      letter-spacing:0.08em;
      background:#1f2438;
      color:#b3b9ff;
    }}
    h1 {{
      margin-top:0.75rem;
      margin-bottom:0.25rem;
    }}
    .metric-row {{
      display:flex;
      gap:1rem;
      margin-top:1rem;
      flex-wrap:wrap;
    }}
    .metric {{
      flex:1 1 160px;
      padding:0.75rem 1rem;
      border-radius:10px;
      background:#15182a;
    }}
    .label {{
      font-size:0.75rem;
      text-transform:uppercase;
      letter-spacing:0.08em;
      opacity:0.75;
    }}
    .value {{
      font-size:1.4rem;
      margin-top:0.25rem;
    }}
    a {{
      color:#8bb4ff;
      text-decoration:none;
    }}
    a:hover {{
      text-decoration:underline;
    }}
    .links {{
      margin-top:1.5rem;
      font-size:0.85rem;
      opacity:0.9;
    }}
    ul {{
      margin:0.35rem 0 0;
      padding-left:1.1rem;
    }}
  </style>
</head>
<body>
  <div class="card">
    <span class="badge">AION · Kernel Node</span>
    <h1>System Snapshot</h1>
    <p>Live view of organism health and awareness currently reported by the AION kernel.</p>

    <div class="metric-row">
      <div class="metric">
        <div class="label">Health</div>
        <div class="value">{health_score:.3}</div>
        <div>{health_label}</div>
      </div>
      <div class="metric">
        <div class="label">Awareness</div>
        <div class="value">{awareness_score:.3}</div>
        <div>{awareness_label}</div>
      </div>
    </div>

    <div class="links">
      <div>JSON / text endpoints:</div>
      <ul>
        <li><a href="/status">/status</a> – health &amp; awareness index (JSON)</li>
        <li><a href="/metrics">/metrics</a> – CPU / memory / IO metrics (JSON)</li>
        <li><a href="/mem">/mem</a> – working memory snapshot (text; cortex policy, etc.)</li>
      </ul>
    </div>
  </div>
</body>
</html>
"#,
        health_score = health_score,
        health_label = health_label,
        awareness_score = awareness_score,
        awareness_label = awareness_label,
    )
}
