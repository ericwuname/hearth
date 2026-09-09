// 行为验证：抽出 O1 数据层 + 供应商层，用极简 DOM 桩真实跑一遍两条 bug 路径
const fs=require('fs'), vm=require('vm');
const s=fs.readFileSync(__dirname+'/codex-desktop-demo.html','utf8');
const js=s.match(/<script[^>]*>([\s\S]*?)<\/script>/)[1];

function slice(a,b){const i=js.indexOf(a), j=js.indexOf(b);
  if(i<0||j<0) throw new Error('marker missing: '+(i<0?a:b));
  return js.slice(i,j);}

const pvBlock = slice("var SEL_PV_ID=''", "function stamp()");
const o1Block = slice('var SB_ARCHIVE=false', "renderSessions('');");

// ---- 极简 DOM 桩：select 有 innerHTML/value，其余返回 null 让守卫短路 ----
function mkSelect(){return {innerHTML:'',value:'',_opts(){return [...this.innerHTML.matchAll(/value="([^"]*)"[^>]*>([^<]*)</g)].map(m=>({v:m[1],t:m[2]}));}};}
const NODES={pvSel:mkSelect(), modelSel:mkSelect(), sbSearch:{value:''},
  pvModels:{textContent:''}, pvName:{value:''}, pvUrl:{value:''}, pvKey:{value:''},
  pvSt:{textContent:'',className:''}, pvList:{innerHTML:'',children:[]}};
const sandbox={
  document:{getElementById:id=>NODES[id]||null, querySelector:()=>null,
            querySelectorAll:()=>[], addEventListener:()=>{}, createElement:()=>({style:{},classList:{add(){},remove(){}}})},
  window:{innerWidth:1400,innerHeight:900},
  setTimeout:()=>0, clearTimeout:()=>{}, console,
  Qseq:"'", esc:x=>String(x), toast:m=>{LOG.push(m);},
  restart:()=>{}, MODE_KEY:'collab', askConfirm:(m,cb)=>cb(),  // 确认框直接放行，测删除后续
};
const LOG=[]; sandbox.LOG=LOG;
vm.createContext(sandbox);
vm.runInContext(o1Block+'\n'+pvBlock, sandbox);
const S=sandbox;
// ⚠ 块内定义的真实 askConfirm 会覆盖桩（它找不到 #sbConfirm 就 return，回调永不执行，
//   导致所有"删除后"的断言空转变成假阳性）。必须在 eval 之后再覆盖一次。
S.askConfirm=(m,cb)=>{LOG.push('[confirm] '+m); cb();};

let fail=0;
const ok=(c,msg)=>{console.log((c?'  PASS  ':'  FAIL  ')+msg); if(!c)fail++;};

console.log('--- BUG1: 新增供应商能否在对话区参选 ---');
S.refreshProviderSel();
ok(NODES.pvSel._opts().length===2, '初始下拉 2 个供应商, 实际 '+NODES.pvSel._opts().length);
S.addProvider();                                  // 设置里新增
const opts=NODES.pvSel._opts();
ok(opts.length===3, '新增后对话区下拉变 3 个, 实际 '+opts.length);
ok(opts[2].t.indexOf('新供应商')===0, '新供应商出现在下拉里: '+opts[2].t);
ok(/未识别/.test(opts[2].t), '未识别模型的供应商有标记提示');
const newId=opts[2].v;
S.PV_ACTIVE=2; S.PROVIDERS[2].url='https://x/v1'; S.PROVIDERS[2].key='sk-x';
S.detectModels();                                  // 识别模型
S.onProvider(newId);                               // 对话区切到新供应商
ok(S.SEL_PV_ID===newId, '可切换到新供应商 (SEL_PV_ID='+S.SEL_PV_ID+')');
ok(NODES.modelSel._opts().length===5, '切过去后模型下拉有 5 项, 实际 '+NODES.modelSel._opts().length);

console.log('--- BUG1b: 删除供应商不得让已有对话指错 ---');
const conv=S.findConv('MSG-d4e5f6');               // 原本指向 pv-2 (Gemini)
const before=conv.provider;
const nBefore=S.PROVIDERS.length;
S.PV_ACTIVE=0; S.delProvider(0);                   // 删掉第一个 (DeepSeek), 下标全部左移
ok(S.PROVIDERS.length===nBefore-1, '[前置] 供应商确实被删除 '+nBefore+'→'+S.PROVIDERS.length);
ok(S.PROVIDERS[0].id!=='pv-1', '[前置] 下标 0 已换成别的供应商 '+S.PROVIDERS[0].id);
ok(conv.provider===before && before==='pv-2', '删首个供应商后, 对话仍指向 '+conv.provider+'（id 引用不漂移）');
S.syncModelSel(conv);
ok(S.SEL_PV_ID==='pv-2', '切回该对话仍解析到 Gemini, 实际 '+S.SEL_PV_ID);

console.log('--- BUG2: 项目里删除的对话能否恢复回原项目 ---');
const P=S.findProj('P-9b8c'); P.open=false;        // 父项目折叠（复现"看不见"）
S.trashConv('MSG-d4e5f6');
ok(conv.trashed===true, '对话已进回收站');
let th=S.trashHtml('');
ok(th.indexOf('项目：sandbox refactor')>=0, '回收站显示来源项目名');
S.SB_TRASH=true;
S.restoreTrash('MSG-d4e5f6');
ok(conv.trashed===false, '恢复后 trashed=false');
ok(P.open===true, '父项目被自动展开（否则看起来没恢复）');
ok(S.SB_TRASH===false, '自动退出回收站视图回到会话视图');
ok(P.convs.some(c=>c.id==='MSG-d4e5f6'), '对话确实回到原项目 P-9b8c 下');
ok(/项目「sandbox refactor」/.test(LOG[LOG.length-1]), 'toast 报出落点: '+LOG[LOG.length-1]);

console.log('--- BUG2b: 空项目进回收站后能否捞回（原先永久丢失）---');
S.newProject();
const emptyId=S.PROJECTS[0].id;
S.trashProject(emptyId);
ok(S.findProj(emptyId).trashed===true, '[前置] 空项目确实进了回收站');
th=S.trashHtml('');
ok(th.indexOf('整个项目')>=0, '空项目在回收站里有独立条目');
ok(th.indexOf('restoreProject(\''+emptyId)>=0, '空项目条目带恢复按钮');
S.restoreProject(emptyId);
ok(S.findProj(emptyId).trashed===false, '空项目成功恢复');

console.log('--- BUG2c: 整项目删除时子对话不应被当成 live ---');
ok(S.liveConvs().some(c=>c.id==='MSG-d4e5f6'), '[前置] 删项目前该对话是 live 的');
S.trashProject('P-9b8c');
ok(S.findProj('P-9b8c').trashed===true, '[前置] 项目确实进了回收站');
ok(!S.liveConvs().some(c=>c.id==='MSG-d4e5f6'), '已删项目下的对话不再计入 live');
ok(S.trashedConvs().every(r=>r.pid!=='P-9b8c'), '已删项目的子对话不重复单列（由项目条目统一恢复）');

console.log('--- BUG3(可达性): 可点击元素不得放进 nowrap+ellipsis 裁剪容器 ---');
// v21d 真凶：.hs / .ht 是 white-space:nowrap;overflow:hidden;text-overflow:ellipsis。
// 回收站把「↩恢复/🗑彻底删」塞进 .hs，来源名一长按钮就被省略号裁掉 ⇒ 逻辑全对但点不到。
// JS 断言测不出布局，所以改成结构断言：裁剪类里出现 onclick 一律判失败。
const CLIP=['hs','ht'];  // 与 CSS 中 nowrap+ellipsis 的类保持一致
function clipViolations(html){
  const bad=[];
  CLIP.forEach(cls=>{
    const re=new RegExp('<div class="'+cls+'"[^>]*>([\\s\\S]*?)<\\/div>','g');
    let m; while((m=re.exec(html))){ if(m[1].indexOf('onclick')>=0) bad.push(cls+': '+m[1].slice(0,60)); }
  });
  return bad;
}
// 自检：断言本身必须能抓出旧写法，否则它只是个永真的空转 PASS
const LEGACY='<div class="hs">项目：sandbox refactor · 剩 7天 <span class="hsb-restore" onclick="restoreTrash(\'a\')">↩恢复</span></div>';
ok(clipViolations(LEGACY).length>0, '[自检] 该断言能抓出旧写法（测试必须能失败）');

// 让回收站里同时存在「整项目」和「长名项目下的单个对话」两类条目
S.restoreProject('P-9b8c');
const LONG=S.findProj('P-9b8c'); LONG.name='sandbox refactor 超长项目名占位测试';
S.trashConv('MSG-d4e5f6');
S.newProject(); S.trashProject(S.PROJECTS[0].id);
const trashHtml=S.trashHtml('');
ok(trashHtml.indexOf('整个项目')>=0 && trashHtml.indexOf('恢复')>=0, '[前置] 回收站同时含项目条目与对话条目');
let bad=clipViolations(trashHtml);
ok(bad.length===0, '回收站按钮不在裁剪容器内'+(bad.length?(' ← 违规: '+bad.join(' | ')):''));
ok(/<div class="hact">/.test(trashHtml), '回收站操作按钮在 .hact 独立行');
ok(/hsb-purge/.test(trashHtml), '彻底删按钮有专属可视样式类（此前是无样式裸文本）');

S.archiveConv('MSG-c2d4e5');
const arcHtml=S.archiveHtml('');
bad=clipViolations(arcHtml);
ok(bad.length===0, '归档按钮不在裁剪容器内'+(bad.length?(' ← 违规: '+bad.join(' | ')):''));
ok(/项目：/.test(arcHtml), '归档条目显示来源项目（恢复落点可见）');

// 侧栏窄，长来源名下按钮必须仍可达：结构上独立成行即满足，这里锁死不得回退成单行拼接
ok(!/<div class="hmeta">[^<]*<span/.test(trashHtml), '元信息行内不得内联按钮（防回退）');

console.log('--- 增强①②：新建对话示例工作区 + 点文件反向联动中央消息 ---');
// 切片第3栏块（含 genSampleWS / rsbJumpToMention / CONV_WS / rsbInit），用 DOM 桩真实跑一遍
const rsbBlock = slice("var RSB_OPEN=false", "rsbInit();");
function mkNode(){return {innerHTML:'',textContent:'',className:'',style:{},dataset:{},classList:{add(){},remove(){},toggle(){}},querySelector:()=>null,querySelectorAll:()=>[],scrollIntoView(){},getAttribute:()=>'',appendChild(){},setAttribute(){},matches:()=>false};}
function mkMsg(mid,text){return {getAttribute:k=>k==='data-mid'?mid:'',querySelector:sel=>sel==='.bubble'?{textContent:text,innerText:text}:null,classList:{add(){},remove(){}},scrollIntoView(){},querySelectorAll:()=>[]};}
const HUMAN={querySelectorAll:sel=>sel==='.msg'?[
  mkMsg('M-c2-1','给数据库加一个 ping 探活探针，连接失败时健康检查返回 503。'),
  mkMsg('M-c2-2','已添加 DBPingProbe（写在 src/db/ping.rs，每 5s 探活，连续 3 次失败标记 unhealthy，/healthz 返回 503），并补了单测 tests/db_ping_test.rs。')
 ]:[]};
const TOAST2=[];
const sb3={
  document:{getElementById:id=>id==='human'?HUMAN:mkNode(),querySelector:()=>mkNode(),querySelectorAll:()=>[],createElement:()=>mkNode(),addEventListener:()=>{},body:{style:{}}},
  window:{innerWidth:1400,innerHeight:900,addEventListener:()=>{}},
  setTimeout:()=>0,clearTimeout:()=>{},console,
  esc:s=>String(s).replace(/[&<>]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;'}[c])),
  toast:m=>{TOAST2.push(m);},
  findConv:id=>({id:id,title:'x',task:'y'}),
  parentOf:()=>null,
  activeConv:()=>({id:'MSG-c2d4e5'}),
  findMsg:id=>null
};
sb3.TOAST2=TOAST2;
vm.createContext(sb3);
vm.runInContext(rsbBlock, sb3);
const S3=sb3;

// ① 新建对话示例工作区（取代空态）
const ws=S3.genSampleWS({title:'T1',task:'做点事'});
ok(ws.files['README.md'] && ws.tree.length===1 && ws.tree[0].k==='README.md', '① 示例工作区含 README.md + 单文件树');
ok(ws.plan.length===3 && ws.plan.every(p=>p.done===false), '① 示例工作区给 3 步空计划骨架');
ok(ws.act.length===1 && ws.arts.length===0, '① 示例工作区活动=等待指令、产物为空');

// ② refs 精确映射：点文件 → 跳到对应消息
S3.CONV_WS['MSG-c2d4e5'].refs={'src/db/ping.rs':'M-c2-2'};
S3.rsbJumpToMention('src/db/ping.rs');
ok(/M-c2-2/.test(TOAST2[TOAST2.length-1]) && /反向联动/.test(TOAST2[TOAST2.length-1]), '② refs 精确跳到 M-c2-2 并提示反向联动');

// ② 文本扫描回退（删 refs 后仍能靠气泡文本命中）
delete S3.CONV_WS['MSG-c2d4e5'].refs;
S3.rsbJumpToMention('tests/db_ping_test.rs');
ok(/M-c2-2/.test(TOAST2[TOAST2.length-1]), '② 无 refs 时文本扫描仍能定位（tests/db_ping_test.rs→M-c2-2）');

// ② 未提及 → 软降级提示（不抛错）
const tb=TOAST2.length;
S3.rsbJumpToMention('Cargo.toml');
ok(TOAST2.length>tb && /未直接提及/.test(TOAST2[TOAST2.length-1]), '② 对话流未提及的文件软降级提示，不抛错');

// ② 对称：点中央消息 → 右栏定位对应文件（双向联动）
S3.CONV_WS['MSG-c2d4e5'].refs={'src/db/ping.rs':'M-c2-2','tests/db_ping_test.rs':'M-c2-2','src/healthz.rs':'M-c2-2'};
S3.msgJumpToFiles('M-c2-2');
ok(/M-c2-2/.test(TOAST2[TOAST2.length-1]) && /src\/db\/ping\.rs/.test(TOAST2[TOAST2.length-1]) && /双向联动/.test(TOAST2[TOAST2.length-1]), '② 对称：点 M-c2-2 定位到右栏 src/db/ping.rs 等并提示双向联动');
const tb2=TOAST2.length;
S3.msgJumpToFiles('M-c2-1'); // 用户任务消息，未关联文件
ok(TOAST2.length>tb2 && /未直接关联/.test(TOAST2[TOAST2.length-1]), '② 对称：无关联文件的消息软降级提示，不抛错');

console.log('--- ① 暗色主题切换（DOM 桩冒烟）---');
const themeBlock = slice("/* ========== THEME-BLOCK-START", "/* ========== THEME-BLOCK-END");
const SB_TH={document:{documentElement:{dataset:{}}, getElementById:()=>null, querySelector:()=>null, querySelectorAll:()=>[], createElement:()=>({})},
  localStorage:{getItem:()=>null,setItem(){},removeItem(){}}, setTimeout:()=>0,clearTimeout:()=>{},console, window:{}};
vm.createContext(SB_TH); vm.runInContext(themeBlock, SB_TH);
const TH=SB_TH;
ok(typeof TH.toggleTheme==='function', '① toggleTheme 已定义（主题切换）');
const r1=TH.toggleTheme();
ok(r1==='dark' && TH.document.documentElement.dataset.theme==='dark', '① 切换→dark 生效（data-theme=dark）');
const r2=TH.toggleTheme();
ok(r2==='' && TH.document.documentElement.dataset.theme==='', '① 再切→回到浅色（data-theme 清空）');
// [自检] 旧写法(不翻转 data-theme)应被上述断言捕获 → 证明断言能失败
const SB_BAD={document:{documentElement:{dataset:{}}}, console}; vm.createContext(SB_BAD);
vm.runInContext('function toggleTheme(){return "";} /* 旧写法：不翻转 data-theme */', SB_BAD);
const rb=SB_BAD.toggleTheme();
ok(!(rb==='dark' && SB_BAD.document.documentElement.dataset.theme==='dark'), '[自检] 断言能抓出旧写法(不翻转 data-theme)');

console.log('--- ② 键盘焦点环（结构断言）---');
ok(/focus-visible/.test(s), '② CSS 含 :focus-visible 焦点环规则（明暗均可见）');
// [自检] 无 focus-visible 的旧写法应被断言捕获
const LEGACY_FV='button{outline:none}';
ok(/focus-visible/.test(LEGACY_FV)===false, '[自检] 断言能抓出无焦点环的旧写法（测试必须能失败）');

console.log('--- ④ reduced-motion（DOM 桩冒烟）---');
// 动画节点在 reduced-motion 下仍渲染（display 不受影响），仅 animation 被关。
// 用真实 CSS 规则校验：媒体查询内对 .port.on/.spin.on/.msg.flash 等置 animation:none
ok(/@media\s*\(prefers-reduced-motion:\s*reduce\)/.test(s), '④ 含 @media prefers-reduced-motion:reduce 块');
ok(/prefers-reduced-motion[\s\S]*?\.port\.on\{animation:none\}/.test(s), '④ 关闭 .port.on 动画（节点仍 display:flex 渲染）');
ok(/prefers-reduced-motion[\s\S]*?\.spin\.on\{animation:none\}/.test(s), '④ 关闭 .spin.on 动画（节点仍 display:inline-flex 渲染）');
ok(/prefers-reduced-motion[\s\S]*?\.msg\.flash,\.hsb-item\.flash\{animation:none\}/.test(s), '④ 关闭 .msg.flash/.hsb-item.flash 闪烁动画');
// [自检] 旧写法（没有关 .spin.on 动画）应被断言捕获 → 证明断言能失败
const LEGACY_RM='@media (prefers-reduced-motion: reduce){.port.on{animation:none}}';
ok(/prefers-reduced-motion[\s\S]*?\.spin\.on\{animation:none\}/.test(LEGACY_RM)===false, '[自检] 断言能抓出旧写法（漏关 .spin.on 动画）');

console.log('--- ⑤ 右侧栏折叠（toggleRsb 行内样式清理 · 防类样式被压）---');
ok(/\.rsb\.collapsed\{flex-basis:0/.test(s), '⑤ .rsb.collapsed 类样式 flex-basis:0 已定义（折叠地基）');
ok(/rsb\.style\.flexBasis=RSB_OPEN\?rsbW\+'px':''/.test(s), '⑤ toggleRsb 关闭时清空行内 flexBasis（否则 .collapsed:0 被行内 340px 压住 → 关不掉）');
// [自检] 旧 bug 写法（仅打开时设行内、关闭不清）应被断言捕获 → 证明断言能失败
const LEGACY_TOGGLE='RSB_OPEN=!RSB_OPEN;rsb.classList.toggle("collapsed",!RSB_OPEN);if(RSB_OPEN)rsb.style.flexBasis=rsbW+"px";';
ok(/rsb\.style\.flexBasis=RSB_OPEN\?rsbW\+'px':''/.test(LEGACY_TOGGLE)===false, '[自检] 断言能抓出旧 bug 写法（关闭不清行内 flexBasis）');

console.log('--- ⑥ 暗色对比度（状态徽章/深底盒 · 防浅字浅底）---');
ok(/\[data-theme="dark"\] \.tag\.done[\s\S]*?background:rgba\(74,222,128,\.16\)/.test(s), '⑥ 暗色下 .tag.done 等绿徽章改深染底（浅绿字可读）');
ok(/\[data-theme="dark"\] \.tag\.warn[\s\S]*?background:rgba\(251,191,36,\.16\)/.test(s), '⑥ 暗色下 .tag.warn 等琥珀徽章改深染底');
ok(/\[data-theme="dark"\] \.tag\.pause[\s\S]*?background:rgba\(248,113,113,\.16\)/.test(s), '⑥ 暗色下 .tag.pause 等红徽章改深染底');
ok(/\[data-theme="dark"\] \.tag\.loop\{background:rgba\(167,139,250,\.18\)/.test(s), '⑥ 暗色下 .tag.loop 紫徽章改深染底');
ok(/\[data-theme="dark"\] \.fnd \.sv\.info\{background:rgba\(139,139,240,\.16\)/.test(s), '⑥ 暗色下 .fnd .sv.info 改深染底（浅字可读）');
ok(/\[data-theme="dark"\] \.modal \.q\{background:#1a1d26;border-color:#3a3550/.test(s), '⑥ 暗色下 .modal .q 显式深底（pur-soft 已浅，避免浅字浅底）');
ok(/\[data-theme="dark"\] \.modal \.why b\{color:#fbbf24/.test(s), '⑥ 暗色下 .modal .why b 改浅琥珀（原 #92400e 深底看不见）');
ok(/--pur-soft:#e9e4ff;/.test(s), '⑥ 暗色下 --pur-soft 统一为浅值（与 accent-soft 一致，浅药丸规则）');
// [自检] 旧写法（pur-soft 暗色仍深 #2a2540、无徽章覆写）应被断言捕获 → 证明断言能失败
const LEGACY_DARK='--pur-soft:#2a2540;';
ok(/--pur-soft:#e9e4ff;/.test(LEGACY_DARK)===false, '[自检] 断言能抓出旧写法（pur-soft 暗色仍为深值 → 浅药丸规则被破坏）');

console.log('--- ⑦ 暗色皮肤层（对比度校验 · 主文字须为浅色且满足可读亮度）---');
ok(/\[data-theme="dark"\][\s\S]*?--tx:#eef1f6/.test(s), '⑦ 暗色主文字 --tx 提亮为 #eef1f6（脆化对比，不发灰）');
ok(/\[data-theme="dark"\][\s\S]*?--mut:#aab2c0/.test(s), '⑦ 暗色次要文字 --mut 提亮为 #aab2c0（可读）');
ok(/\[data-theme="dark"\][\s\S]*?--bd:#333a47/.test(s), '⑦ 暗色边框 --bd 提亮为 #333a47（分隔可见）');
ok(/\[data-theme="dark"\] \.card,[\s\S]*?\.modal\{color:var\(--tx\)\}/.test(s), '⑦ 暗色皮肤层：主要容器显式 深底+浅字（不依赖继承）');
// [自检] 主文字若退回深值（暗底上看不见）应被断言捕获
const LEGACY_TX='--tx:#1f2328;';
ok(/--tx:#eef1f6/.test(LEGACY_TX)===false, '[自检] 断言能抓出旧写法（暗色主文字退回深值 → 看不见）');

console.log('--- ⑧ 暗色输入框/选中项/差异行/联动高亮 对比度（防浅字浅底）---');
// #1 输入框在暗色下须有显式浅字：<textarea> 不继承 body 颜色，UA 默认 fieldtext=黑 → 黑字压黑底
ok(/#huInput\{[^}]*color:var\(--tx\)/.test(s), '⑧ 暗色下 #huInput 显式浅字（不再黑字压黑底）');
// #2 左栏选中项（临时对话/项目下对话）暗色下改深染底，否则 -soft 浅药丸压浅字看不见
ok(/\[data-theme="dark"\] \.hsb-item\.on\{background:rgba\(139,139,240,\.18\)/.test(s), '⑧ 暗色下左栏选中项改深染底（浅字可读）');
// #3 差异行 .dl.del 暗色下改深染底 + 浅代码字（此前漏改 → 白底浅字看不见）；.dl.add 已由 ⑥ 覆写
ok(/\[data-theme="dark"\] \.dl\.del\{background:rgba\(248,113,113,\.16\)/.test(s), '⑧ 暗色下 .dl.del 改深染底（红差异行可读）');
ok(/\[data-theme="dark"\] \.dl\.del \.ds\{color:var\(--tx\)/.test(s), '⑧ 暗色下差异代码字显式浅色（不靠继承）');
ok(/\[data-theme="dark"\] \.dl\.add \.ds\{color:var\(--tx\)/.test(s), '⑧ 暗色下 .dl.add 代码字显式浅色');
// #4 联动高亮改用主题 accent（表“已联动”而非“成功/绿”），去易误读绿
ok(/@keyframes hsbflash\{[^}]*var\(--accent\)/.test(s), '⑧ 联动高亮 hsbflash 改用主题 accent');
ok(/@keyframes hsbflash\{[^}]*#dcfce7/.test(s)===false, '⑧ 联动高亮 hsbflash 已不含绿色 #dcfce7');
// [自检] 旧写法应被断言捕获 → 证明断言能失败
const LEGACY_IN='#huInput{background:var(--bg)}';                                            // 无显式 color
ok(/#huInput\{[^}]*color:var\(--tx\)/.test(LEGACY_IN)===false, '[自检] 断言能抓出旧写法（输入框无显式颜色）');
const LEGACY_SEL='[data-theme="dark"] .hsb-item.on{background:var(--accent-soft)}';         // 浅药丸压浅字
ok(/\[data-theme="dark"\] \.hsb-item\.on\{background:rgba\(139,139,240,\.18\)/.test(LEGACY_SEL)===false, '[自检] 断言能抓出旧写法（选中项仍浅药丸）');
const LEGACY_DEL='.dl.del{background:#fdecec}.dl.del .ds{color:#1f2328}';                     // 暗色下白底 → 实际浅字看不见
ok(/\[data-theme="dark"\] \.dl\.del\{background:rgba\(248,113,113,\.16\)/.test(LEGACY_DEL)===false, '[自检] 断言能抓出旧写法（.dl.del 仍白底）');
const LEGACY_FL='@keyframes hsbflash{0%,60%{background:#dcfce7;border-color:#16a34a}}';       // 绿
ok(/@keyframes hsbflash\{[^}]*#dcfce7/.test(LEGACY_FL)===true, '[自检] 断言能抓出旧写法（高亮仍绿色 → 被 no-green 断言判失败）');
// ⑧a 绿差异行须带 data-theme 前缀的暗染底（压过后文浅色 .dl.add{background:#e9f9ef}）
ok(/\[data-theme="dark"\] \.dl\.add\{background:rgba\(74,222,128,\.16\)/.test(s), '⑧ 暗色下 .dl.add 带前缀暗染底（压过浅色规则 → 绿行可读）');
// ⑧b 规划草案高亮步骤行 .ds.hit 暗色改深染底（原为 -soft 浅药丸压浅字）
ok(/\[data-theme="dark"\] \.draft \.ds\.hit\{background:rgba\(139,139,240,\.18\)/.test(s), '⑧ 暗色下 .draft .ds.hit 改深染底（浅字可读）');
// [自检] 旧写法（绿差异行无前缀暗染底 / 草案高亮仍浅药丸）应被断言捕获
const LEGACY_DLA='.dl.add{background:#e9f9ef}.dl.add .dm{color:var(--ok)}';                     // 暗色下仍是浅底
ok(/\[data-theme="dark"\] \.dl\.add\{background:rgba\(74,222,128,\.16\)/.test(LEGACY_DLA)===false, '[自检] 断言能抓出旧写法（绿差异行无带前缀暗染底）');
const LEGACY_DH='.draft .ds.hit{background:var(--accent-soft)}';                                 // 浅药丸压浅字
ok(/\[data-theme="dark"\] \.draft \.ds\.hit\{background:rgba\(139,139,240,\.18\)/.test(LEGACY_DH)===false, '[自检] 断言能抓出旧写法（草案高亮仍浅药丸）');
// ⑧c 设置弹窗选中供应商行 .pv-row.on 暗色改深染底（原为 -soft 浅药丸压浅字）
ok(/\[data-theme="dark"\] \.pv-row\.on\{background:rgba\(139,139,240,\.18\)/.test(s), '⑧ 暗色下 .pv-row.on 改深染底（设置弹窗选中行浅字可读）');
// [自检] 旧写法（设置弹窗选中行仍浅药丸）应被断言捕获
const LEGACY_PV='.pv-row.on{background:var(--accent-soft);border:1px solid var(--accent)}';       // 浅药丸压浅字
ok(/\[data-theme="dark"\] \.pv-row\.on\{background:rgba\(139,139,240,\.18\)/.test(LEGACY_PV)===false, '[自检] 断言能抓出旧写法（设置弹窗选中行仍浅药丸）');

console.log('--- ③ 空态 / 加载态（结构断言）---');
// 硬约束：.rsb-empty-ws 类名与触发契约不得变。校验 5 处触发字符串仍含该类名。
const emptyHits=(s.match(/rsb-empty-ws/g)||[]).length;
ok(emptyHits>=5, '③ .rsb-empty-ws 类名仍在源码（触发契约 intact，实际 '+emptyHits+' 处）');
ok(/class="stream" id="human"[^>]*>\s*<\/div>/.test(s), '③ 中央 .stream#human 容器结构 intact（空态皮肤可兜底）');
ok(/\.skeleton::after[\s\S]*?@keyframes skel/.test(s), '③ 加载态 skeleton 抖动态已定义（纯 CSS 工具类）');
// [自检] 旧写法（.rsb-empty-ws 被改名）应被断言捕获 → 证明断言能失败
const LEGACY_EMPTY='<div class="rsb-empty">该对话暂无产物</div>';
ok(/rsb-empty-ws/.test(LEGACY_EMPTY)===false, '[自检] 断言能抓出旧写法（.rsb-empty-ws 被改名）');

console.log('--- R1-① 键盘导航可达性（分屏手柄须可聚焦）---');
// 分屏拖拽手柄 #hsbDiv / #rsbDiv 须键盘可达（tabindex=0 + role=separator），锚点 id 不动
ok(/id="hsbDiv"[^>]*\btabindex="0"/.test(s) && /id="hsbDiv"[^>]*role="separator"/.test(s), 'R1-① #hsbDiv 键盘可达（tabindex=0 + role=separator）');
ok(/id="rsbDiv"[^>]*\btabindex="0"/.test(s) && /id="rsbDiv"[^>]*role="separator"/.test(s), 'R1-① #rsbDiv 键盘可达（tabindex=0 + role=separator）');
ok(/:focus-visible/.test(s), 'R1-① :focus-visible 焦点环规则仍在（继承 #01 ②）');
// 静态可聚焦/可交互元素数量（native focusable tags 代理枚举；JS 动态生成控件同走 native 标签）
const BTN=(s.match(/<button/g)||[]).length;
ok(BTN>=8, 'R1-① 静态可交互按钮 ≥8（实际 '+BTN+'，Tab 可达基数）');
// [自检] 旧写法（手柄 tabindex=-1 不可聚焦 / 无焦点环）应被断言捕获 → 证明断言能失败
const LEGACY_H='<div class="hsb-div" id="hsbDiv" tabindex="-1"></div>';
ok(/id="hsbDiv"[^>]*\btabindex="0"/.test(LEGACY_H)===false, '[自检] 断言能抓出旧写法（手柄 tabindex=-1 不可聚焦）');
ok(/:focus-visible/.test('button{outline:none}')===false, '[自检] 断言能抓出无焦点环旧写法');

console.log('--- R1-② WCAG 2.1 AA 对比度量化（sRGB→相对亮度→CR）---');
function hx(h){h=h.replace('#','');if(h.length===3)h=h.split('').map(c=>c+c).join('');return [parseInt(h.slice(0,2),16),parseInt(h.slice(2,4),16),parseInt(h.slice(4,6),16)];}
function rel(c){const f=t=>t<=0.03928?t/12.92:Math.pow((t+0.055)/1.055,2.4);const[r,g,b]=hx(c).map(x=>x/255);return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b);}
function CR(a,b){const L1=rel(a),L2=rel(b);return (Math.max(L1,L2)+0.05)/(Math.min(L1,L2)+0.05);}
function vars(block){const o={};const re=/--([a-z0-9-]+):\s*(#[0-9a-fA-F]{3,6})/g;let m;while((m=re.exec(block)))o[m[1]]=m[2];return o;}
const LV=vars(s.match(/:root\{([^}]*)\}/)[1]);
const DV=vars(s.match(/\[data-theme="dark"\]\{([^}]*)\}/)[1]);
const AA=4.5, AA_L=3;
ok(CR(LV.tx,LV.bg)>=AA, 'R1-② 浅色 --tx/--bg '+CR(LV.tx,LV.bg).toFixed(2)+':1 ≥ 4.5');
ok(CR(LV.mut,LV.bg)>=AA, 'R1-② 浅色 --mut/--bg '+CR(LV.mut,LV.bg).toFixed(2)+':1 ≥ 4.5');
ok(CR(LV.accent,LV.bg)>=AA, 'R1-② 浅色 --accent/--bg '+CR(LV.accent,LV.bg).toFixed(2)+':1 ≥ 4.5');
ok(CR(DV.tx,DV.bg)>=AA, 'R1-② 暗色 --tx/--bg '+CR(DV.tx,DV.bg).toFixed(2)+':1 ≥ 4.5');
ok(CR(DV.mut,DV.bg)>=AA, 'R1-② 暗色 --mut/--bg '+CR(DV.mut,DV.bg).toFixed(2)+':1 ≥ 4.5');
ok(CR(DV.accent,DV.bg)>=AA, 'R1-② 暗色 --accent/--bg '+CR(DV.accent,DV.bg).toFixed(2)+':1 ≥ 4.5');
// 暗色徽章合成（rgba 叠 card）折算：.tag.done/warn 文字=语义色 on 暗染底
function over(fg,a,bg){const[r,g,b]=hx(fg),[R,G,B]=hx(bg);return '#'+[0,1,2].map(i=>Math.round([r,g,b][i]*a+[R,G,B][i]*(1-a)).toString(16).padStart(2,'0')).join('');}
ok(CR(DV.ok,over(DV.ok,0.16,DV.card))>=AA_L, 'R1-② 暗色 .tag.done 文字/合成底 '+CR(DV.ok,over(DV.ok,0.16,DV.card)).toFixed(2)+':1 ≥ 3');
ok(CR(DV.warn,over(DV.warn,0.16,DV.card))>=AA_L, 'R1-② 暗色 .tag.warn 文字/合成底 '+CR(DV.warn,over(DV.warn,0.16,DV.card)).toFixed(2)+':1 ≥ 3');
// [自检] 故意不达标旧写法（深字压深底）应被断言捕获 → 证明量化能抓问题，非空转
ok(CR('#1f2328','#0c0e12')<AA, '[自检] 断言能抓出深字压深底（对比度不达标）');

console.log('--- R2-④ 响应式断点（@media 断点值存在且真生效）---');
ok(/@media\s*\(max-width:\s*768px\)/.test(s), 'R2-④ 含 ≤768px 断点（右栏默认收起）');
ok(/@media\s*\(max-width:\s*480px\)/.test(s), 'R2-④ 含 ≤480px 断点（三栏堆叠，对话优先）');
ok(/@media\s*\(max-width:\s*480px\)[\s\S]*?#portHuman\.on\{flex-direction:column/.test(s), 'R2-④ ≤480px 下 #portHuman.on 转纵向堆叠');
// [自检] 旧写法（无任何 @media 断点）应被断言捕获
const LEGACY_R='.port.on{flex-direction:row}';
ok(/max-width:\s*768px/.test(LEGACY_R)===false && /max-width:\s*480px/.test(LEGACY_R)===false, '[自检] 断言能抓出旧写法（无 @media 断点）');

console.log(fail? ('\nRESULT: '+fail+' FAILED') : '\nRESULT: ALL PASS');
process.exit(fail?1:0);
