// Local-only preferences; no analytics, network calls or tracking cookies.
(() => {
  const applyLanguage = (language) => {
    const lang = language === 'fr' ? 'fr' : 'en';
    document.documentElement.lang = lang;
    document.querySelectorAll('[data-fr][data-en]').forEach((element) => {
      element.textContent = element.dataset[lang];
    });
    document.querySelectorAll('[data-language]').forEach((button) => {
      button.setAttribute('aria-pressed', String(button.dataset.language === lang));
    });
    try { localStorage.setItem('regta-language', lang); } catch (_) { /* Private browsing. */ }
  };
  document.querySelectorAll('[data-language]').forEach((button) => {
    button.addEventListener('click', () => applyLanguage(button.dataset.language));
  });
  let language = 'en';
  try { language = localStorage.getItem('regta-language') || 'en'; } catch (_) { /* English fallback. */ }
  applyLanguage(language);
  document.querySelectorAll('.languages').forEach((group) => { group.hidden = false; });

  // GitHub Pages supplies the real account and repository; no invented username.
  const host = location.hostname;
  const repo = location.pathname.split('/').filter(Boolean)[0];
  if (/^[a-z0-9-]+\.github\.io$/i.test(host) && repo === 'regta-ctw') {
    const owner = host.slice(0, -'.github.io'.length);
    document.querySelectorAll('.repo-link').forEach((link) => {
      link.href = `https://github.com/${owner}/${repo}`;
      link.hidden = false;
    });
  }
})();
