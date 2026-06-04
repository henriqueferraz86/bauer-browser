//! Mode policies — JS scripts injected per mode.

pub const MODES: &[&str] = &["lite", "normal", "full"];

/// JS injected at page load for Lite and Normal modes — blocks autoplay.
pub const BLOCK_AUTOPLAY_JS: &str = r#"(function(){
    'use strict';
    function muteEl(el){el.autoplay=false;el.muted=true;try{el.pause();}catch(_){}}
    document.querySelectorAll('video,audio').forEach(muteEl);
    new MutationObserver(function(ms){
        ms.forEach(function(m){
            m.addedNodes.forEach(function(n){
                if(!n||n.nodeType!==1)return;
                if(n.tagName==='VIDEO'||n.tagName==='AUDIO')muteEl(n);
                n.querySelectorAll&&n.querySelectorAll('video,audio').forEach(muteEl);
            });
        });
    }).observe(document.documentElement,{childList:true,subtree:true});
})();"#;

/// JS injected in Lite mode — attempts to block 3rd-party script injection.
pub const BLOCK_3RD_PARTY_SCRIPTS_JS: &str = r#"(function(){
    'use strict';
    var host=window.location.hostname;
    var _orig=document.createElement.bind(document);
    document.createElement=function(tag){
        var el=_orig(tag);
        if(tag.toLowerCase()!=='script')return el;
        var _set=el.setAttribute.bind(el);
        el.setAttribute=function(n,v){
            if(n==='src'){
                try{
                    var u=new URL(v,window.location.href);
                    if(u.hostname&&u.hostname!==host&&!u.hostname.endsWith('.'+host)){
                        console.log('[Bauer Lite] blocked:',v);return;
                    }
                }catch(_){}
            }
            return _set(n,v);
        };
        return el;
    };
})();"#;

/// JS that extracts readable content and rewrites the page.
pub const READER_MODE_JS: &str = r#"(function(){
    var sels=['article','main','[role="main"]','.post-content',
              '.article-body','.entry-content','.content','#content','.post'];
    var root=null;
    for(var i=0;i<sels.length;i++){root=document.querySelector(sels[i]);if(root)break;}
    if(!root)root=document.body;
    var c=root.cloneNode(true);
    c.querySelectorAll(
        'script,style,nav,header,footer,aside,iframe,.ad,.ads,'+
        '.advertisement,.sidebar,[class*="banner"],[class*="popup"]'
    ).forEach(function(e){e.remove();});
    var css='body{font-family:Georgia,serif;font-size:19px;line-height:1.85;'+
        'max-width:720px;margin:48px auto;padding:0 24px 80px;color:#2c3e50;background:#fafafa}'+
        'h1,h2,h3{color:#1a252f}a{color:#2980b9}'+
        'img{max-width:100%;border-radius:6px;margin:20px 0}'+
        'blockquote{border-left:4px solid #3498db;padding:.5em 1em;background:#eaf4fb;font-style:italic}'+
        'pre{background:#f0f0f0;padding:16px;border-radius:6px;overflow-x:auto}'+
        'code{background:#f0f0f0;padding:2px 6px;border-radius:3px}';
    var html='<!DOCTYPE html><html><head><meta charset="utf-8">'+
        '<meta name="viewport" content="width=device-width,initial-scale=1">'+
        '<title>'+document.title+'</title><style>'+css+'</style></head><body>'+
        '<h1>'+document.title+'</h1><div>'+c.innerHTML+'</div></body></html>';
    document.open();document.write(html);document.close();
})();"#;

pub fn autoplay_script_for_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "lite" | "normal" => Some(BLOCK_AUTOPLAY_JS),
        _ => None,
    }
}

pub fn extra_script_for_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "lite" => Some(BLOCK_3RD_PARTY_SCRIPTS_JS),
        _ => None,
    }
}

pub fn mode_color(mode: &str) -> &'static str {
    match mode {
        "lite"   => "#2ecc71",
        "normal" => "#3498db",
        "full"   => "#e74c3c",
        _        => "#888888",
    }
}
