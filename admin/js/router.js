const pages = {};
let activePage = null;
let refreshInterval = null;

function registerPage(name, callbacks) {
  pages[name] = callbacks;
}

function switchPage(name) {
  if (activePage === name) return;
  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));

  const pageEl = document.getElementById('page-' + name);
  const btnEl = document.querySelector(`[data-page="${name}"]`);
  if (pageEl) pageEl.classList.add('active');
  if (btnEl) btnEl.classList.add('active');

  activePage = name;
  if (refreshInterval) clearInterval(refreshInterval);

  if (pages[name]) {
    if (pages[name].init) pages[name].init();
    if (pages[name].refresh) {
      pages[name].refresh();
      refreshInterval = setInterval(pages[name].refresh, 5000);
    }
  }
}

document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('.nav-btn').forEach(btn => {
    btn.addEventListener('click', () => switchPage(btn.dataset.page));
  });

  document.getElementById('nodeSelect').addEventListener('change', () => {
    if (pages[activePage] && pages[activePage].refresh) {
      pages[activePage].refresh();
    }
  });

  switchPage('nodes');
});
