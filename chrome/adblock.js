// Bauer Browser — Ad & Tracker Blocker
// Runs in ALL frames (including ad iframes).
// __BAUER_BLOCKED__ must be defined before this script (injected by Rust).

(function () {
    'use strict';

    var BLOCKED = (typeof __BAUER_BLOCKED__ !== 'undefined') ? __BAUER_BLOCKED__ : [];
    if (BLOCKED.length === 0) return;

    // ── URL check ─────────────────────────────────────────────────────────────
    function isBlocked(url) {
        if (!url || typeof url !== 'string') return false;
        var lower;
        try {
            lower = new URL(url, document.baseURI).href.toLowerCase();
        } catch (_) {
            lower = url.toLowerCase();
        }
        for (var i = 0; i < BLOCKED.length; i++) {
            if (lower.indexOf(BLOCKED[i]) !== -1) return true;
        }
        return false;
    }

    // ── Block fetch() ─────────────────────────────────────────────────────────
    if (typeof window.fetch === 'function') {
        var _fetch = window.fetch;
        window.fetch = function (resource) {
            var url = (typeof resource === 'string') ? resource
                    : (resource && resource.url) ? resource.url : '';
            if (isBlocked(url)) {
                return Promise.reject(new TypeError('[Bauer] Blocked: ' + url));
            }
            return _fetch.apply(this, arguments);
        };
    }

    // ── Block XMLHttpRequest ──────────────────────────────────────────────────
    if (typeof XMLHttpRequest !== 'undefined') {
        var _open = XMLHttpRequest.prototype.open;
        XMLHttpRequest.prototype.open = function (method, url) {
            if (isBlocked(url)) { this.__bauerBlocked = true; return; }
            return _open.apply(this, arguments);
        };
        var _send = XMLHttpRequest.prototype.send;
        XMLHttpRequest.prototype.send = function () {
            if (this.__bauerBlocked) return;
            return _send.apply(this, arguments);
        };
    }

    // ── Block dynamic createElement (script / iframe / img / link) ────────────
    var _create = document.createElement.bind(document);
    document.createElement = function (tag) {
        var el   = _create(tag);
        var tl   = typeof tag === 'string' ? tag.toLowerCase() : '';
        if (tl === 'script' || tl === 'iframe' || tl === 'img' || tl === 'link') {
            var _set = el.setAttribute.bind(el);
            el.setAttribute = function (name, value) {
                if ((name === 'src' || name === 'href') && isBlocked(value)) return;
                return _set(name, value);
            };
        }
        return el;
    };

    // ── Remove / hide ad elements via MutationObserver ────────────────────────
    var AD_CLASS_PATTERNS = [
        'adsbygoogle', 'ad-container', 'advertisement', 'sponsored-content',
        'google-ad', 'dfp-', 'mgid', 'taboola', 'outbrain',
    ];

    function shouldHide(node) {
        if (!node || node.nodeType !== 1) return false;
        var id  = (node.id  || '').toLowerCase();
        var cls = typeof node.className === 'string' ? node.className.toLowerCase() : '';
        for (var i = 0; i < AD_CLASS_PATTERNS.length; i++) {
            if (id.indexOf(AD_CLASS_PATTERNS[i]) !== -1 ||
                cls.indexOf(AD_CLASS_PATTERNS[i]) !== -1) return true;
        }
        return false;
    }

    function processNode(node) {
        if (!node || node.nodeType !== 1) return;
        // Block by src/href
        var src = node.src || node.href ||
                  (node.getAttribute ? node.getAttribute('src') : null);
        if (src && isBlocked(src)) {
            if (node.parentNode) node.parentNode.removeChild(node);
            return;
        }
        // Hide by class/id pattern
        if (shouldHide(node)) {
            node.style.setProperty('display', 'none', 'important');
        }
    }

    new MutationObserver(function (mutations) {
        mutations.forEach(function (m) {
            m.addedNodes.forEach(processNode);
        });
    }).observe(document.documentElement, { childList: true, subtree: true });

    // Scan DOM once it's ready
    function scanDom() {
        document.querySelectorAll('script,iframe,img,link,ins,div').forEach(processNode);
    }
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', scanDom);
    } else {
        scanDom();
    }

    // ── CSS — hide known ad containers ────────────────────────────────────────
    var CSS_SELECTORS = [
        '.adsbygoogle',
        '[id*="google_ads"]', '[class*="google-ad"]',
        '[id*="ad-slot"]',    '[class*="ad-slot"]',
        '[id*="dfp-"]',       '[class*="dfp-ad"]',
        '[class*="advertisement"]', '[class*="sponsored"]',
        '.taboola-widget',    '.OUTBRAIN',
        '[id*="taboola"]',    '[id*="outbrain"]',
        '[class*="mgid"]',    '[id*="mgid"]',
        'ins.adsbygoogle',
    ];

    try {
        if (!document.getElementById('__bauer_adblock_css__')) {
            var s = _create('style');
            s.id  = '__bauer_adblock_css__';
            s.textContent = CSS_SELECTORS.join(',') + '{display:none!important}';
            (document.head || document.documentElement).appendChild(s);
        }
    } catch (_) {}

})();
