/* Lampion animated mark, shared by the artboards. Plain script, works from file://.
   Geometry is docs/brand/lampion/mark.svg, split into groups so parts can move:
   .lp-swing rotates about the hanger point (24, 1.5); .lp-core is the pale centre. */
(function () {
  const SVG = `
<svg viewBox="0 0 48 48" aria-hidden="true" focusable="false">
  <g class="lp-swing"><g class="lp-sway">
    <path d="M24 1.5V7" stroke="#D9820B" stroke-width="2.5" stroke-linecap="round"/>
    <rect x="16.5" y="6.75" width="15" height="5.5" rx="1.75" fill="#D9820B" stroke="#1F2347" stroke-width="2"/>
    <rect x="18.5" y="37.75" width="11" height="5.5" rx="1.75" fill="#D9820B" stroke="#1F2347" stroke-width="2"/>
    <ellipse cx="24" cy="25" rx="18" ry="14" fill="#FFB81C" stroke="#1F2347" stroke-width="2.5"/>
    <ellipse class="lp-core" cx="24" cy="25.5" rx="8.5" ry="7" fill="#FFF1C7"/>
    <path d="M8.5 19.25Q24 22 39.5 19.25M8.5 30.75Q24 33.5 39.5 30.75" fill="none" stroke="#C76E00" stroke-width="1.6" stroke-linecap="round" opacity=".8"/>
  </g></g>
</svg>`;
  const CHECK = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg>';

  const prefersReduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function mount(host, opts = {}) {
    host.classList.add("lp");
    host.style.setProperty("--size", (opts.size || 64) + "px");
    host.innerHTML = `<div class="lp-halo"></div><div class="lp-rings"></div>${SVG}<div class="lp-check">${CHECK}</div>`;
    host.dataset.state = opts.state || "hidden";
    host.dataset.mood = opts.mood || "alert";
    const rings = host.querySelector(".lp-rings");

    let target = 0, level = 0, last = performance.now(), raf = 0, frozen = false;
    const reduced = () => prefersReduced() || !!host.closest(".reduce-motion");

    function apply() {
      const st = host.dataset.state;
      const r = reduced();
      if (st === "listening") {
        host.style.setProperty("--core-s", r ? 1 : (1 + 0.35 * level).toFixed(3));
        host.style.setProperty("--core-o", r ? (0.78 + 0.22 * level).toFixed(3) : 1);
        host.style.setProperty("--halo", (0.3 + 0.7 * level).toFixed(3));
      } else if (st === "speaking") {
        host.style.setProperty("--core-s", r ? 1 : (1 + 0.22 * level).toFixed(3));
        host.style.setProperty("--core-o", (0.82 + 0.18 * level).toFixed(3));
        host.style.setProperty("--halo", (0.25 + 0.6 * level).toFixed(3));
      } else {
        ["--core-s", "--core-o", "--halo"].forEach(p => host.style.removeProperty(p));
        if (st === "armed") host.style.setProperty("--halo", ".22");
        if (st === "inserted") host.style.setProperty("--halo", ".3");
      }
    }
    function tick(now) {
      const dt = now - last; last = now;
      level += (target - level) * (1 - Math.exp(-dt / 60)); // ~60 ms smoothing
      apply();
      raf = requestAnimationFrame(tick);
    }
    function ensureLoop() { if (!raf && !frozen) { last = performance.now(); raf = requestAnimationFrame(tick); } }

    const api = {
      el: host,
      get state() { return host.dataset.state; },
      set(state, extra = {}) {
        if (extra.mood) host.dataset.mood = extra.mood;
        host.classList.remove("is-pulse");
        host.dataset.state = state;
        if (state !== "listening" && state !== "speaking") target = level = 0;
        apply();
        ensureLoop();
        return api;
      },
      mood(m) { host.dataset.mood = m; return api; },
      level(v) { target = Math.max(0, Math.min(1, v)); ensureLoop(); return api; },
      flicker() {
        host.classList.add("is-flicker");
        clearTimeout(host._fl);
        host._fl = setTimeout(() => host.classList.remove("is-flicker"), 120);
        return api;
      },
      ring() {
        if (reduced()) return api;
        const d = document.createElement("div");
        d.className = "lp-ring";
        rings.appendChild(d);
        setTimeout(() => d.remove(), 1400);
        return api;
      },
      pulse() {
        host.classList.remove("is-pulse");
        void host.offsetWidth;
        host.classList.add("is-pulse");
        setTimeout(() => host.classList.remove("is-pulse"), 560);
        return api;
      },
      /* Deterministic pose for screenshots: no animation, fixed values. */
      freeze(pose = {}) {
        frozen = true; cancelAnimationFrame(raf); raf = 0;
        host.style.animation = "none";
        host.querySelectorAll(".lp-swing,.lp-sway,.lp-core,.lp-halo").forEach(n => (n.style.animation = "none"));
        if (pose.level != null) { level = target = pose.level; apply(); }
        if (pose.rotate != null && !reduced()) host.querySelector(".lp-swing").style.transform = `rotate(${pose.rotate}deg)`;
        if (pose.ghost != null && !reduced()) {
          /* storyboard convention: a faint copy at the other end of the motion */
          const g = host.querySelector("svg").cloneNode(true);
          g.style.opacity = ".28";
          g.querySelector(".lp-swing").style.transform = `rotate(${pose.ghost}deg)`;
          host.insertBefore(g, host.querySelector("svg"));
        }
        if (pose.coreOpacity != null) host.style.setProperty("--core-o", pose.coreOpacity);
        if (pose.halo != null) host.style.setProperty("--halo", pose.halo);
        (pose.rings || []).forEach(s => {
          if (reduced()) return;
          const d = document.createElement("div");
          d.className = "lp-ring";
          d.style.animation = "none";
          d.style.transform = `scale(${s})`;
          d.style.opacity = String(Math.max(0, 1 - (s - 0.7) / 1.6) * 0.85);
          rings.appendChild(d);
        });
        return api;
      },
    };
    apply();
    return api;
  }

  /* A speech-like envelope: syllables (~5 Hz) inside phrases separated by short pauses. */
  function speechEnvelope(seed = 1) {
    let s = seed;
    const rnd = () => ((s = (s * 16807) % 2147483647) / 2147483647);
    let phraseEnd = 0, pauseEnd = 0, syll = 0, sylLen = 180, amp = 0.7, newPhrase = false;
    return function sample(t) {
      if (t < pauseEnd) return { v: 0.04 + 0.03 * rnd(), phraseStart: false };
      if (t >= phraseEnd) {
        if (phraseEnd !== 0 && t < phraseEnd + 10) { /* fallthrough */ }
        pauseEnd = t + 220 + rnd() * 380;
        phraseEnd = pauseEnd + 1100 + rnd() * 1500;
        newPhrase = true;
        return { v: 0.05, phraseStart: false };
      }
      if (t - syll > sylLen) { syll = t; sylLen = 130 + rnd() * 140; amp = 0.45 + rnd() * 0.55; }
      const p = (t - syll) / sylLen;
      const v = amp * Math.sin(Math.PI * Math.min(1, p)) * (0.85 + 0.15 * rnd());
      const start = newPhrase; newPhrase = false;
      return { v, phraseStart: start };
    };
  }

  window.Lampion = { mount, speechEnvelope, prefersReduced };
})();
