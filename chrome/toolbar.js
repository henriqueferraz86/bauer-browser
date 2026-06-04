// Bauer Browser — Toolbar injection script
// Runs via with_initialization_script on every page load.
// Creates a fixed-position toolbar at the top of every page.

(function () {
    'use strict';

    // Only inject in the top-level frame — skip iframes (ads, embeds, etc.)
    if (window !== window.top) return;

    var TB_ID   = '__bauer_toolbar__';
    var TB_H    = 56; // px

    // ── IPC ───────────────────────────────────────────────────────────────
    window.__bsend = function (action, extra) {
        try {
            window.ipc.postMessage(
                JSON.stringify(Object.assign({ action: action }, extra || {}))
            );
        } catch (e) { console.error('[Bauer IPC]', e); }
    };

    window.__bnav = function () {
        var el = document.getElementById('__bauer_url__');
        if (el && el.value.trim()) {
            window.__bsend('navigate', { url: el.value.trim() });
        }
    };

    // ── Called from Rust via evaluate_script ──────────────────────────────
    window.__bauerUpdateUrl = function (url) {
        var el = document.getElementById('__bauer_url__');
        if (el) el.value = url;
    };

    window.__bauerUpdateRam = function (mb) {
        var el = document.getElementById('__bauer_ram__');
        if (!el) return;
        el.textContent = mb.toFixed(0) + ' MB';
        el.style.color = mb > 300 ? '#f7768e' : '#565f89';
    };

    window.__bauerShowAgent = function (text) {
        var panel = document.getElementById('__bauer_agent_panel__');
        var txt   = document.getElementById('__bauer_agent_text__');
        var btn   = document.getElementById('__bauer_agent_btn__');
        if (txt)   txt.textContent = text;
        if (panel) panel.style.display = 'block';
        if (btn)   { btn.textContent = '🤖 Resumir'; btn.disabled = false; }
    };

    // ── Inject toolbar ─────────────────────────────────────────────────────
    function inject() {
        if (document.getElementById(TB_ID)) return;
        if (!document.body) return;

        // Styles
        var style = document.createElement('style');
        style.textContent = [
            '#' + TB_ID + '{',
            '  position:fixed;top:0;left:0;right:0;height:' + TB_H + 'px;',
            '  background:#1e2030;z-index:2147483647;',
            '  display:flex;align-items:center;gap:5px;padding:0 8px;',
            '  box-shadow:0 2px 10px rgba(0,0,0,.5);',
            '  font-family:system-ui,-apple-system,sans-serif;',
            '}',
            '#' + TB_ID + ' .bb{',
            '  background:#2a2d3e;color:#c0caf5;border:1px solid #414868;',
            '  border-radius:6px;padding:0 10px;height:30px;cursor:pointer;',
            '  font-size:15px;white-space:nowrap;',
            '}',
            '#' + TB_ID + ' .bb:hover{background:#414868}',
            '#__bauer_url__{',
            '  flex:1;background:#2a2d3e;color:#c0caf5;border:1px solid #414868;',
            '  border-radius:7px;padding:0 10px;height:30px;font-size:13px;outline:none;',
            '}',
            '#__bauer_url__:focus{border-color:#7aa2f7}',
            '#__bauer_url__::placeholder{color:#565f89}',
            '#__bauer_mode__{',
            '  background:#2a2d3e;color:#c0caf5;border:1px solid #414868;',
            '  border-radius:6px;padding:0 6px;height:30px;font-size:12px;cursor:pointer;',
            '}',
            '#__bauer_ram__{font-size:11px;color:#565f89;min-width:55px;text-align:right;}',
            '#__bauer_agent_panel__{',
            '  display:none;position:fixed;bottom:0;right:0;width:360px;',
            '  max-height:220px;overflow-y:auto;',
            '  background:#1a1c2a;border:1px solid #414868;border-radius:10px 0 0 0;',
            '  padding:12px;font-size:13px;color:#c0caf5;',
            '  z-index:2147483646;box-shadow:-4px -4px 20px rgba(0,0,0,.4);',
            '}',
            '#__bauer_agent_header__{',
            '  display:flex;justify-content:space-between;',
            '  margin-bottom:8px;font-weight:600;color:#7aa2f7;',
            '}',
            '#__bauer_agent_close__{cursor:pointer;color:#565f89;font-size:14px;}',
            '#__bauer_agent_close__:hover{color:#c0caf5}',
            '#__bauer_agent_text__{white-space:pre-wrap;line-height:1.6}',
        ].join('');
        document.head.appendChild(style);

        // Toolbar
        var tb = document.createElement('div');
        tb.id = TB_ID;
        tb.innerHTML = [
            '<button class="bb" onclick="__bsend(\'back\')" title="Back (Alt+←)">←</button>',
            '<button class="bb" onclick="__bsend(\'forward\')" title="Forward (Alt+→)">→</button>',
            '<button class="bb" onclick="__bsend(\'reload\')" title="Reload (F5)">↺</button>',
            '<input id="__bauer_url__" type="text" placeholder="URL or search…"',
            '       onkeydown="if(event.key===\'Enter\')__bnav()">',
            '<select id="__bauer_mode__" class="bb"',
            '        onchange="__bsend(\'setMode\',{mode:this.value})" title="Page mode">',
            '  <option value="lite">🌿 Lite</option>',
            '  <option value="normal" selected>⚡ Normal</option>',
            '  <option value="full">🔥 Full</option>',
            '</select>',
            '<button class="bb" onclick="__bsend(\'readerMode\')" title="Reader Mode">📖</button>',
            '<button id="__bauer_agent_btn__" class="bb"',
            '        style="font-size:12px;min-width:82px;"',
            '        onclick="this.textContent=\'⏳…\';this.disabled=true;__bsend(\'agentSummarize\')">',
            '  🤖 Resumir',
            '</button>',
            '<span id="__bauer_ram__">-- MB</span>',
        ].join('');
        document.body.insertBefore(tb, document.body.firstChild);

        // Agent result panel
        var panel = document.createElement('div');
        panel.id = '__bauer_agent_panel__';
        panel.innerHTML = [
            '<div id="__bauer_agent_header__">',
            '  🤖 Bauer Agent — Resumo',
            '  <span id="__bauer_agent_close__"',
            '        onclick="document.getElementById(\'__bauer_agent_panel__\').style.display=\'none\'">✕</span>',
            '</div>',
            '<div id="__bauer_agent_text__"></div>',
        ].join('');
        document.body.appendChild(panel);

        // Push page content below toolbar
        var curr = parseInt(window.getComputedStyle(document.body).paddingTop) || 0;
        if (curr < TB_H) {
            document.body.style.paddingTop = TB_H + 'px';
        }
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', inject);
    } else {
        inject();
    }

    // ── Keyboard shortcuts ─────────────────────────────────────────────────
    document.addEventListener('keydown', function (e) {
        if (e.altKey && e.key === 'ArrowLeft')  { __bsend('back');    e.preventDefault(); }
        if (e.altKey && e.key === 'ArrowRight') { __bsend('forward'); e.preventDefault(); }
        if (e.key === 'F5') { __bsend('reload'); e.preventDefault(); }
        if (e.ctrlKey && e.key === 'l') {
            var el = document.getElementById('__bauer_url__');
            if (el) { el.focus(); el.select(); e.preventDefault(); }
        }
    });

})();
