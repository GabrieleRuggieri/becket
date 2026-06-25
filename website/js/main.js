/* ===== Theme Toggle ===== */
function initTheme() {
  const saved = localStorage.getItem("becket-theme") || "dark";
  document.documentElement.setAttribute("data-theme", saved);

  const btn = document.getElementById("theme-toggle");
  if (!btn) return;

  btn.addEventListener("click", () => {
    const current = document.documentElement.getAttribute("data-theme");
    const next = current === "dark" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", next);
    localStorage.setItem("becket-theme", next);
    document.dispatchEvent(new CustomEvent("themechange"));
  });
}

/* ===== Particle Network (optimized) ===== */
function initParticles() {
  const canvas = document.getElementById("particles");
  if (!canvas) return;

  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reducedMotion) {
    canvas.style.display = "none";
    return;
  }

  const ctx = canvas.getContext("2d", { alpha: true, desynchronized: true });
  if (!ctx) return;

  let particles = [];
  let animId = null;
  let running = false;
  let dotColor = "";
  let lineColor = "";
  const count = window.innerWidth < 768 ? 40 : 70;
  const connectionDistSq = 140 * 140;

  function cacheColors() {
    const light = document.documentElement.getAttribute("data-theme") === "light";
    dotColor = light ? "rgba(249, 115, 22, 0.25)" : "rgba(249, 115, 22, 0.4)";
    lineColor = light ? "rgba(249, 115, 22, 0.12)" : "rgba(249, 115, 22, 0.12)";
  }

  function resize() {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.floor(window.innerWidth * dpr);
    canvas.height = Math.floor(window.innerHeight * dpr);
    canvas.style.width = window.innerWidth + "px";
    canvas.style.height = window.innerHeight + "px";
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  function createParticles() {
    const w = window.innerWidth;
    const h = window.innerHeight;
    particles = [];
    for (let i = 0; i < count; i++) {
      particles.push({
        x: Math.random() * w,
        y: Math.random() * h,
        vx: (Math.random() - 0.5) * 0.4,
        vy: (Math.random() - 0.5) * 0.4,
        r: Math.random() * 1.5 + 0.5,
      });
    }
  }

  function draw() {
    if (!running) return;

    const w = window.innerWidth;
    const h = window.innerHeight;
    ctx.clearRect(0, 0, w, h);

    ctx.beginPath();
    ctx.fillStyle = dotColor;
    for (const p of particles) {
      p.x += p.vx;
      p.y += p.vy;
      if (p.x < 0 || p.x > w) p.vx *= -1;
      if (p.y < 0 || p.y > h) p.vy *= -1;
      ctx.moveTo(p.x + p.r, p.y);
      ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
    }
    ctx.fill();

    for (let i = 0; i < particles.length; i++) {
      const a = particles[i];
      for (let j = i + 1; j < particles.length; j++) {
        const b = particles[j];
        const dx = a.x - b.x;
        const dy = a.y - b.y;
        const distSq = dx * dx + dy * dy;
        if (distSq < connectionDistSq) {
          const alpha = 1 - Math.sqrt(distSq) / 140;
          ctx.globalAlpha = alpha;
          ctx.beginPath();
          ctx.moveTo(a.x, a.y);
          ctx.lineTo(b.x, b.y);
          ctx.strokeStyle = lineColor;
          ctx.lineWidth = 0.5;
          ctx.stroke();
        }
      }
    }
    ctx.globalAlpha = 1;

    animId = requestAnimationFrame(draw);
  }

  function start() {
    if (running) return;
    running = true;
    cacheColors();
    draw();
  }

  function stop() {
    running = false;
    if (animId !== null) {
      cancelAnimationFrame(animId);
      animId = null;
    }
  }

  cacheColors();
  resize();
  createParticles();

  document.addEventListener("visibilitychange", () => {
    document.hidden ? stop() : start();
  });

  document.addEventListener("themechange", cacheColors);

  let resizeTimer;
  window.addEventListener(
    "resize",
    () => {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        resize();
        createParticles();
      }, 150);
    },
    { passive: true }
  );

  if (!document.hidden) start();
}

/* ===== Typing Effect (runs only when visible) ===== */
function initTyping() {
  const output = document.getElementById("typing-output");
  const terminal = output?.closest(".hero-terminal, .terminal");
  if (!output || !terminal) return;

  let lineIndex = 0;
  let charIndex = 0;
  let currentText = "";
  let timeout;
  let active = false;

  function type() {
    if (!active) return;

    const lines = window.getTypingLines();
    if (lineIndex >= lines.length) {
      timeout = setTimeout(() => {
        output.textContent = "";
        lineIndex = 0;
        charIndex = 0;
        currentText = "";
        type();
      }, 3000);
      return;
    }

    const line = lines[lineIndex];

    if (charIndex < line.length) {
      currentText += line[charIndex];
      charIndex++;
      output.textContent = currentText;
      timeout = setTimeout(type, line.startsWith("$") ? 40 : 18);
    } else {
      currentText += "\n";
      output.textContent = currentText;
      lineIndex++;
      charIndex = 0;
      timeout = setTimeout(type, line === "" ? 200 : 350);
    }
  }

  window.restartTyping = () => {
    clearTimeout(timeout);
    output.textContent = "";
    lineIndex = 0;
    charIndex = 0;
    currentText = "";
    if (active) type();
  };

  const observer = new IntersectionObserver(
    ([entry]) => {
      active = entry.isIntersecting;
      if (active) {
        type();
      } else {
        clearTimeout(timeout);
      }
    },
    { threshold: 0.1 }
  );

  observer.observe(terminal);
}

/* ===== Scroll Reveal ===== */
function initReveal() {
  const els = document.querySelectorAll(".reveal");
  if (!els.length) return;

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const delay = entry.target.dataset.delay || 0;
          setTimeout(() => entry.target.classList.add("visible"), Number(delay));
          observer.unobserve(entry.target);
        }
      });
    },
    { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
  );

  els.forEach((el) => observer.observe(el));
}

/* ===== Nav Scroll & Mobile ===== */
function initNav() {
  const nav = document.getElementById("nav");
  const burger = document.getElementById("nav-burger");
  const links = document.getElementById("nav-links") || document.querySelector(".nav-links");

  const setMenuOpen = (open) => {
    burger?.classList.toggle("active", open);
    links?.classList.toggle("open", open);
    document.body.classList.toggle("nav-open", open);
    burger?.setAttribute("aria-expanded", open ? "true" : "false");
    if (open && links) {
      const first = links.querySelector("a");
      first?.focus();
    }
  };

  const focusableInMenu = () =>
    links
      ? [...links.querySelectorAll('a, button, [tabindex]:not([tabindex="-1"])')].filter(
          (el) => !el.disabled && el.offsetParent !== null
        )
      : [];

  let ticking = false;
  window.addEventListener(
    "scroll",
    () => {
      if (!ticking) {
        requestAnimationFrame(() => {
          if (nav) nav.classList.toggle("scrolled", window.scrollY > 20);
          ticking = false;
        });
        ticking = true;
      }
    },
    { passive: true }
  );

  if (burger && links) {
    burger.setAttribute("aria-expanded", "false");
    burger.setAttribute("aria-controls", "nav-links");
    burger.addEventListener("click", () => {
      setMenuOpen(!links.classList.contains("open"));
    });

    links.querySelectorAll("a").forEach((a) => {
      a.addEventListener("click", () => setMenuOpen(false));
    });

    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape") setMenuOpen(false);
      if (e.key !== "Tab" || !links.classList.contains("open")) return;
      const items = focusableInMenu();
      if (!items.length) return;
      const first = items[0];
      const last = items[items.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    });
  }
}

/* ===== Homepage section scroll spy ===== */
function initScrollSpy() {
  const navLinks = document.querySelectorAll('.nav-links a[href^="#"]');
  if (!navLinks.length) return;

  const sections = [...navLinks]
    .map((a) => {
      const id = a.getAttribute("href")?.slice(1);
      return id ? document.getElementById(id) : null;
    })
    .filter(Boolean);

  if (!sections.length) return;

  const setActive = (id) => {
    navLinks.forEach((a) => {
      a.classList.toggle("active", a.getAttribute("href") === `#${id}`);
    });
  };

  const observer = new IntersectionObserver(
    (entries) => {
      const visible = entries
        .filter((e) => e.isIntersecting)
        .sort((a, b) => b.intersectionRatio - a.intersectionRatio);
      if (visible[0]) setActive(visible[0].target.id);
    },
    { rootMargin: "-40% 0px -50% 0px", threshold: [0, 0.15, 0.35] }
  );

  sections.forEach((s) => observer.observe(s));
}

/* ===== Docs: Active TOC & Copy ===== */
function initDocs() {
  const tocLinks = document.querySelectorAll(".docs-toc a");
  const sections = document.querySelectorAll(".doc-section");

  if (tocLinks.length && sections.length) {
    const sectionObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const id = entry.target.id;
            tocLinks.forEach((a) => {
              a.classList.toggle("active", a.getAttribute("href") === `#${id}`);
            });
          }
        });
      },
      { threshold: 0.3, rootMargin: "-80px 0px -50% 0px" }
    );

    sections.forEach((s) => sectionObserver.observe(s));
  }

  document.querySelectorAll(".copy-btn").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const text = btn.dataset.copy;
      try {
        await navigator.clipboard.writeText(text);
        const original = btn.textContent;
        btn.textContent = "✓";
        const live = document.getElementById("copy-live");
        if (live) live.textContent = "Copied to clipboard";
        setTimeout(() => {
          btn.textContent = original;
          if (live) live.textContent = "";
        }, 1500);
      } catch {
        btn.textContent = "!";
      }
    });
  });

  document.querySelectorAll(".table-wrap").forEach((wrap) => {
    const check = () => wrap.classList.toggle("has-overflow", wrap.scrollWidth > wrap.clientWidth + 2);
    check();
    wrap.addEventListener("scroll", check, { passive: true });
    window.addEventListener("resize", check, { passive: true });
  });
}

/* ===== Smooth anchor highlight ===== */
function initSmoothAnchors() {
  document.querySelectorAll('a[href^="#"]').forEach((anchor) => {
    anchor.addEventListener("click", (e) => {
      const target = document.querySelector(anchor.getAttribute("href"));
      if (target) {
        e.preventDefault();
        target.scrollIntoView({ behavior: "smooth" });
      }
    });
  });
}

/* ===== Init ===== */
document.addEventListener("DOMContentLoaded", () => {
  initTheme();
  initParticles();
  initTyping();
  initReveal();
  initNav();
  initScrollSpy();
  initDocs();
  initSmoothAnchors();
});
