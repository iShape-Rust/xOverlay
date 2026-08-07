(() => {
  "use strict";
  const report = JSON.parse(document.getElementById("benchmark-data").textContent);
  const state = { coordinate: "i32", output: "shapes" };
  const colors = {
    "iOverlay · single": "#307cf6",
    "iOverlay · multi": "#8557d9",
    "xOverlay · single": "#058b7d",
    "xOverlay · multi": "#ef4444",
    "Boost Polygon 90 · single": "#f28c28"
  };

  const seriesName = measurement => {
    const mode = measurement.execution === "multi_thread" ? "multi" : "single";
    return `${measurement.implementation} · ${mode}`;
  };

  const formatTime = ns => {
    if (ns < 1_000) return `${ns.toFixed(0)} ns`;
    if (ns < 1_000_000) return `${(ns / 1_000).toFixed(ns < 10_000 ? 2 : 1)} µs`;
    if (ns < 1_000_000_000) return `${(ns / 1_000_000).toFixed(ns < 10_000_000 ? 2 : 1)} ms`;
    return `${(ns / 1_000_000_000).toFixed(2)} s`;
  };

  const escapeHtml = text => String(text).replace(/[&<>"]/g, char => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;"
  })[char]);

  function renderMetadata() {
    const metadata = report.metadata;
    const rows = [
      ["Machine", metadata.cpu],
      ["Versions", `iO ${metadata.i_overlay_version} · xO ${metadata.x_overlay_version} · Boost ${metadata.boost_version}`]
    ];
    document.getElementById("run-metadata").innerHTML = rows.map(([key, value]) =>
      `<div><dt>${escapeHtml(key)}</dt><dd>${escapeHtml(value || "unknown")}</dd></div>`
    ).join("");
  }

  function setupControls(id, key) {
    document.querySelectorAll(`#${id} button`).forEach(button => {
      button.addEventListener("click", () => {
        state[key] = button.dataset.value;
        document.querySelectorAll(`#${id} button`).forEach(candidate =>
          candidate.setAttribute("aria-pressed", String(candidate === button))
        );
        renderSuite();
      });
    });
  }

  function scenarioMeasurements(id) {
    return report.measurements.filter(measurement =>
      measurement.scenario === id &&
      measurement.coordinate_type === state.coordinate &&
      measurement.output_kind === state.output
    );
  }

  function renderSuite() {
    const suite = document.getElementById("suite");
    suite.innerHTML = report.scenarios.map((scenario, index) => {
      const measurements = scenarioMeasurements(scenario.id);
      const warning = measurements.find(item => item.warning)?.warning;
      return `
        <article class="scenario" id="${escapeHtml(scenario.id)}">
          <div class="scenario-copy">
            <h2><span class="scenario-index">${String(index + 1).padStart(2, "0")}</span>${escapeHtml(scenario.label)}</h2>
            <p class="scenario-description">${escapeHtml(scenario.description)}</p>
            <span class="operation">${escapeHtml(scenario.operation)}</span>
            <img class="illustration" src="${escapeHtml(scenario.illustration)}" alt="${escapeHtml(scenario.label)} subject and clip geometry">
          </div>
          <div class="result-panel">
            ${warning ? `<p class="warning">${escapeHtml(warning)}</p>` : ""}
            ${measurements.length ? renderResults(scenario.id, measurements) : `<div class="empty">No measurements for this selection.</div>`}
          </div>
        </article>`;
    }).join("");
    report.scenarios.forEach(scenario => {
      const measurements = scenarioMeasurements(scenario.id);
      if (measurements.length) drawChart(document.getElementById(`chart-${scenario.id}`), measurements);
    });
  }

  function renderResults(id, measurements) {
    const names = [...new Set(measurements.map(seriesName))];
    const nValues = [...new Set(measurements.map(item => item.n))].sort((a, b) => a - b);
    const tableRows = nValues.map(n => {
      const cells = names.map(name => {
        const item = measurements.find(candidate => candidate.n === n && seriesName(candidate) === name);
        if (!item) return "<td>—</td>";
        const detail = `${item.output.contours.toLocaleString()} contours · ${item.output.points.toLocaleString()} points`;
        return `<td title="${escapeHtml(detail)}">${formatTime(item.timing.median_ns)}</td>`;
      }).join("");
      return `<tr><td>${n.toLocaleString()}</td>${cells}</tr>`;
    }).join("");
    return `
      <div class="chart-shell"><canvas id="chart-${escapeHtml(id)}" role="img" aria-label="${escapeHtml(id)} performance chart"></canvas></div>
      <div class="legend">${names.map(name => `<span class="legend-item"><span class="legend-line ${name.startsWith("Boost") ? "dashed" : ""}" style="border-color:${colors[name] || "#18201e"}"></span>${escapeHtml(name)}</span>`).join("")}</div>
      <div class="table-wrap"><table>
        <thead><tr><th>N</th>${names.map(name => `<th>${escapeHtml(name)}</th>`).join("")}</tr></thead>
        <tbody>${tableRows}</tbody>
      </table></div>`;
  }

  function drawChart(canvas, measurements) {
    const bounds = canvas.getBoundingClientRect();
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(bounds.width * ratio));
    canvas.height = Math.max(1, Math.round(bounds.height * ratio));
    const context = canvas.getContext("2d");
    context.scale(ratio, ratio);
    const width = bounds.width;
    const height = bounds.height;
    const padding = { left: 64, right: 22, top: 28, bottom: 46 };
    const plotWidth = width - padding.left - padding.right;
    const plotHeight = height - padding.top - padding.bottom;
    const xs = measurements.map(item => Math.log2(item.n));
    const ys = measurements.map(item => Math.log10(Math.max(1, item.timing.median_ns)));
    const xMin = Math.min(...xs), xMax = Math.max(...xs);
    const yMin = Math.floor(Math.min(...ys)), yMax = Math.ceil(Math.max(...ys));
    const xScale = value => padding.left + ((Math.log2(value) - xMin) / Math.max(1, xMax - xMin)) * plotWidth;
    const yScale = value => padding.top + (1 - (Math.log10(Math.max(1, value)) - yMin) / Math.max(1, yMax - yMin)) * plotHeight;

    context.font = "11px ui-monospace, monospace";
    context.fillStyle = "#68716e";
    context.strokeStyle = "#dedbd2";
    context.lineWidth = 1;
    for (let power = yMin; power <= yMax; power++) {
      const y = yScale(10 ** power);
      context.beginPath(); context.moveTo(padding.left, y); context.lineTo(width - padding.right, y); context.stroke();
      context.textAlign = "right"; context.textBaseline = "middle";
      context.fillText(formatTime(10 ** power), padding.left - 10, y);
    }
    const nValues = [...new Set(measurements.map(item => item.n))].sort((a, b) => a - b);
    nValues.forEach(n => {
      const x = xScale(n);
      context.textAlign = "center"; context.textBaseline = "top";
      context.fillText(String(n), x, height - padding.bottom + 12);
    });
    context.textAlign = "right"; context.fillText("N", width - padding.right, height - 13);

    [...new Set(measurements.map(seriesName))].forEach(name => {
      const series = measurements.filter(item => seriesName(item) === name).sort((a, b) => a.n - b.n);
      context.strokeStyle = colors[name] || "#18201e";
      context.fillStyle = context.strokeStyle;
      context.lineWidth = 2.5;
      context.setLineDash(name.startsWith("Boost") ? [7, 5] : []);
      context.beginPath();
      series.forEach((item, index) => {
        const x = xScale(item.n), y = yScale(item.timing.median_ns);
        if (index === 0) context.moveTo(x, y); else context.lineTo(x, y);
      });
      context.stroke();
      context.setLineDash([]);
      series.forEach(item => {
        const x = xScale(item.n), y = yScale(item.timing.median_ns);
        context.beginPath(); context.arc(x, y, 3.5, 0, Math.PI * 2); context.fill();
      });
    });
  }

  let resizeTimer;
  window.addEventListener("resize", () => {
    window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(renderSuite, 120);
  });
  renderMetadata();
  setupControls("coordinate-filter", "coordinate");
  setupControls("output-filter", "output");
  renderSuite();
})();
