import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { Archive, ArrowLeft, Check, ChevronDown, Clock3, Code2, FileImage, FilePlus2, FolderOpen, ImagePlus, Link2, List, ListOrdered, LoaderCircle, MoreHorizontal, Moon, Plus, Search, Settings2, Sun, Text, Trash2, X } from 'lucide-react';
import { EditorContent, NodeViewWrapper, ReactNodeViewRenderer, useEditor } from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import Image from '@tiptap/extension-image';
import Link from '@tiptap/extension-link';
import Placeholder from '@tiptap/extension-placeholder';
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
import { all, createLowlight } from 'lowlight';

const lowlight = createLowlight(all);
const emptyDoc = { type: 'doc', content: [{ type: 'paragraph' }] };
const isCapture = () => getCurrentWindow().label === 'capture';
const imageUrl = (path) => path ? convertFileSrc(path) : '';
const prettyDate = (date) => new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(new Date(date));
const errorText = (e) => typeof e === 'string' ? e : 'Something went wrong. Please try again.';
const LocalImage = Image.extend({
  addNodeView() {
    return ReactNodeViewRenderer(({ node, selected }) => <NodeViewWrapper className="image-node"><img src={imageUrl(node.attrs.src)} alt={node.attrs.alt || 'Screenshot'} className={selected ? 'selected' : ''} /></NodeViewWrapper>);
  },
});

function App() {
  const captureWindow = isCapture();
  const [page, setPage] = useState('home');
  const [workspaces, setWorkspaces] = useState([]);
  const [active, setActive] = useState(null);
  const [search, setSearch] = useState('');
  const [settings, setSettings] = useState({ watch_dir: '' });
  const [dark, setDark] = useState(false);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [captureItems, setCaptureItems] = useState([]);
  const [captureIndex, setCaptureIndex] = useState(0);
  const [drop, setDrop] = useState(false);

  const refresh = useCallback(async (q = '') => {
    try { setWorkspaces(await invoke('list_workspaces', { query: q.trim() || null })); }
    catch (e) { setError(errorText(e)); }
  }, []);
  useEffect(() => { refresh(search); }, [refresh, search]);
  useEffect(() => { invoke('get_settings').then(setSettings).catch(() => {}); }, []);
  useEffect(() => {
    let unlisteners = [];
    Promise.all([
      listen('screenshot-detected', (e) => { setCaptureItems((items) => [...items, e.payload]); setCaptureIndex((i) => captureItems.length ? i : 0); getCurrentWindow().label !== 'capture' && invoke('show_capture').catch(() => {}); }),
      listen('new-workspace', () => createEmpty()),
      listen('open-settings', () => setPage('settings')),
    ]).then((fns) => { unlisteners = fns; });
    return () => unlisteners.forEach((f) => f());
  }, []);
  useEffect(() => { document.documentElement.dataset.theme = dark ? 'dark' : 'light'; }, [dark]);
  useEffect(() => {
    if (!captureWindow) return;
    const current = getCurrentWindow();
    const off = current.onDragDropEvent((event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') setDrop(true);
      if (event.payload.type === 'leave') setDrop(false);
      if (event.payload.type === 'drop') {
        setDrop(false);
        const paths = event.payload.paths.filter((p) => /\.(png|jpe?g|webp)$/i.test(p));
        if (paths.length) { setCaptureItems((items) => [...items, ...paths]); setCaptureIndex(captureItems.length); }
      }
    });
    return () => { off.then((f) => f()); };
  }, [captureWindow, captureItems.length]);

  const createEmpty = async (paths = []) => {
    setBusy(true); setError('');
    try { const workspace = await invoke('create_workspace', { title: paths.length ? 'Untitled workspace' : 'Untitled workspace', imagePaths: paths }); setActive(workspace); setPage('workspace'); if (captureWindow) closeCaptureItem(); await refresh(search); }
    catch (e) { setError(errorText(e)); } finally { setBusy(false); }
  };
  const openWorkspace = async (id) => { try { setActive(await invoke('get_workspace', { id })); setPage('workspace'); } catch (e) { setError(errorText(e)); } };
  const closeCaptureItem = async () => { setCaptureItems((items) => items.filter((_, i) => i !== captureIndex)); setCaptureIndex(0); if (captureWindow && captureItems.length <= 1) await getCurrentWindow().hide(); };
  const save = useCallback(async (title, document) => {
    if (!active) return;
    try { const updated = await invoke('save_workspace', { input: { id: active.id, title: title || 'Untitled workspace', document } }); setActive(updated); setWorkspaces((rows) => [updated, ...rows.filter((r) => r.id !== updated.id)]); setError(''); }
    catch (e) { setError(errorText(e)); throw e; }
  }, [active]);
  const changeWatchDir = async () => {
    const selected = await open({ directory: true, multiple: false, title: 'Choose screenshot folder' });
    if (!selected) return;
    try { await invoke('set_watch_directory', { path: selected }); setSettings({ watch_dir: selected }); setError(''); }
    catch (e) { setError(errorText(e)); }
  };

  if (captureWindow) return <CapturePopup item={captureItems[captureIndex]} count={captureItems.length} index={captureIndex} setIndex={setCaptureIndex} onCreate={createEmpty} onDismiss={closeCaptureItem} busy={busy} drop={drop} onError={setError} error={error} clearError={() => setError('')} />;
  if (page === 'workspace' && active) return <Workspace key={active.id} workspace={active} onBack={() => { setPage('home'); refresh(search); }} onSave={save} onDelete={async () => { await invoke('delete_workspace', { id: active.id }); setActive(null); setPage('home'); refresh(search); }} onError={setError} error={error} clearError={() => setError('')} />;
  if (page === 'settings') return <Settings settings={settings} onChoose={changeWatchDir} dark={dark} setDark={setDark} onBack={() => setPage('home')} />;
  return <Home workspaces={workspaces} search={search} setSearch={setSearch} openWorkspace={openWorkspace} create={() => createEmpty()} settings={() => setPage('settings')} error={error} clearError={() => setError('')} />;
}

function Home({ workspaces, search, setSearch, openWorkspace, create, settings, error, clearError }) {
  return <main className="home-shell">
    <aside className="rail"><div className="brand-mark"><Archive size={17} strokeWidth={2.2} /></div><div className="rail-bottom"><button className="icon-button" title="Settings" onClick={settings}><Settings2 size={17}/></button><div className="avatar">Q</div></div></aside>
    <section className="library">
      <header className="topbar"><div className="crumb"><span className="brand-name">Quickspace</span><span className="slash">/</span><span>Library</span></div><button className="settings-link" onClick={settings}><Settings2 size={15}/> Settings</button></header>
      <div className="home-content"><div className="welcome"><div><p className="eyebrow">YOUR PERSONAL LIBRARY</p><h1>Keep the context.</h1><p className="subhead">A quiet place for the things worth remembering.</p></div><button className="primary-button" onClick={create}><Plus size={16}/> New workspace</button></div>
      <div className="toolbar"><label className="search-box"><Search size={16}/><input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search your workspaces"/><kbd>⌘ K</kbd></label><div className="sort-label"><Clock3 size={14}/> Recently edited</div></div>
      {error && <div className="notice error-notice"><span>{error}</span><button onClick={clearError}><X size={15}/></button></div>}
      {workspaces.length ? <div className="workspace-grid">{workspaces.map((w) => <WorkspaceCard key={w.id} workspace={w} onClick={() => openWorkspace(w.id)} />)}</div> : <div className="empty-state"><div className="empty-art"><ImagePlus size={25}/></div><h2>{search ? 'Nothing found' : 'Start with a screenshot'}</h2><p>{search ? 'Try another search term.' : 'Capture something useful, then give it a little context.'}</p><button className="quiet-button" onClick={create}><Plus size={15}/> Create a workspace</button></div>}
      <footer className="home-footer"><span>{workspaces.length} {workspaces.length === 1 ? 'workspace' : 'workspaces'}</span><span><span className="pulse-dot"/> Stored on this Mac</span></footer></div>
    </section>
  </main>;
}

function WorkspaceCard({ workspace, onClick }) {
  const imgs = useMemo(() => { const found = []; const walk = (node) => { if (node?.type === 'image' && node.attrs?.src) found.push(node.attrs.src); (node?.content || []).forEach(walk); }; walk(workspace.document); return found; }, [workspace.document]);
  return <button className="workspace-card" onClick={onClick}><div className="card-image">{imgs[0] ? <img src={imageUrl(imgs[0])} /> : <div className="card-placeholder"><Text size={20}/></div>}<span className="card-open">↗</span></div><div className="card-meta"><strong>{workspace.title || 'Untitled workspace'}</strong><span>{prettyDate(workspace.updated_at)}</span></div><p>{workspace.preview || 'No notes yet'}</p></button>;
}

function CapturePopup({ item, count, index, setIndex, onCreate, onDismiss, busy, drop, onError, error, clearError }) {
  const [spaces, setSpaces] = useState([]); const [menu, setMenu] = useState(false);
  useEffect(() => { invoke('list_workspaces', { query: null }).then(setSpaces).catch(() => {}); }, []);
  useEffect(() => { if (!item && count === 0) return; }, [item, count]);
  const addTo = async (w) => {
    try { const path = await invoke('add_image', { path: item, workspaceId: w.id }); const doc = JSON.parse(JSON.stringify(w.document)); doc.content ||= []; doc.content.push({ type: 'image', attrs: { src: path, alt: 'Screenshot' } }, { type: 'paragraph' }); await invoke('save_workspace', { input: { id: w.id, title: w.title, document: doc } }); onDismiss(); }
    catch (e) { onError(errorText(e)); }
  };
  const filename = item?.split(/[\\/]/).pop() || 'Drop an image to get started';
  return <div className={`capture-wrap ${drop ? 'is-dropping' : ''}`}>
    <div className="capture-card"><div className="capture-top"><div className="capture-brand"><span className="tiny-mark"><Archive size={13}/></span><span>Quick capture</span></div><div className="capture-actions">{count > 1 && <div className="capture-pager"><button onClick={() => setIndex((index + count - 1) % count)}>‹</button><span>{index + 1}/{count}</span><button onClick={() => setIndex((index + 1) % count)}>›</button></div>}<button className="icon-button small" onClick={onDismiss} title="Dismiss"><X size={15}/></button></div></div>
    <div className="capture-preview">{item ? <img src={imageUrl(item)} alt="Screenshot preview"/> : <div className="capture-empty"><FileImage size={24}/><span>Drop a screenshot here</span></div>}{drop && <div className="drop-overlay">Drop to add screenshot</div>}</div>
    {error && <div className="capture-error"><span>{error}</span><button onClick={clearError}><X size={12}/></button></div>}
    <div className="capture-file"><FileImage size={14}/><span title={filename}>{filename}</span><span className="file-state">Ready</span></div>
    <div className="capture-bottom"><button className="primary-button compact" disabled={!item || busy} onClick={() => onCreate([item])}>{busy ? <LoaderCircle className="spin" size={15}/> : <Plus size={15}/>} New workspace</button><div className="add-existing"><button className="secondary-button compact" disabled={!item} onClick={() => setMenu(!menu)}>Add to… <ChevronDown size={14}/></button>{menu && <div className="popover">{spaces.length ? spaces.slice(0, 7).map((w) => <button key={w.id} onClick={() => addTo(w)}>{w.title || 'Untitled workspace'}</button>) : <span>No workspaces yet</span>}</div>}</div></div>
    {count === 0 && <div className="capture-hint">Drop images from Finder · ⌥⇧S to open</div>}
    </div>{drop && <div className="drop-border"/>}</div>;
}

function Workspace({ workspace, onBack, onSave, onDelete, onError, error, clearError }) {
  const [title, setTitle] = useState(workspace.title); const [saveState, setSaveState] = useState('Saved'); const [savedAt, setSavedAt] = useState(workspace.updated_at); const [menu, setMenu] = useState(false); const [deleteConfirm, setDeleteConfirm] = useState(false);
  const editor = useEditor({ extensions: [StarterKit.configure({ codeBlock: false }), CodeBlockLowlight.configure({ lowlight }), LocalImage.configure({ inline: false, allowBase64: false }), Link.configure({ openOnClick: false, autolink: true }), Placeholder.configure({ placeholder: 'Write down why this is worth remembering…' })], content: workspace.document || emptyDoc, editorProps: { attributes: { class: 'tiptap' } }, onUpdate: ({ editor }) => { setSaveState('Saving…'); } });
  useEffect(() => { if (!editor) return; const t = setTimeout(async () => { try { await onSave(title, editor.getJSON()); setSaveState('Saved'); setSavedAt(new Date().toISOString()); } catch { setSaveState('Could not save'); } }, 650); return () => clearTimeout(t); }, [editor, title, editor?.state.doc]);
  useEffect(() => {
    if (!editor) return;
    const current = getCurrentWindow();
    const off = current.onDragDropEvent(async (event) => {
      if (event.payload.type !== 'drop') return;
      const paths = event.payload.paths.filter((p) => /\.(png|jpe?g|webp)$/i.test(p));
      for (const path of paths) { try { const imported = await invoke('add_image', { path, workspaceId: workspace.id }); editor.chain().focus().setImage({ src: imported, alt: 'Screenshot' }).run(); } catch (e) { onError(errorText(e)); } }
    }); return () => { off.then((f) => f()); };
  }, [editor, workspace.id]);
  const pickImages = async () => { const paths = await open({ multiple: true, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] }); if (!paths) return; for (const path of Array.isArray(paths) ? paths : [paths]) { try { const src = await invoke('add_image', { path, workspaceId: workspace.id }); editor.chain().focus().setImage({ src, alt: 'Screenshot' }).run(); } catch (e) { onError(errorText(e)); } } };
  const setLink = () => { const previous = editor.getAttributes('link').href; const url = window.prompt('Paste a link', previous || 'https://'); if (url === null) return; if (url === '') editor.chain().focus().extendMarkRange('link').unsetLink().run(); else editor.chain().focus().extendMarkRange('link').setLink({ href: url }).run(); };
  return <main className="editor-shell"><header className="editor-topbar"><div className="editor-left"><button className="back-button" onClick={onBack}><ArrowLeft size={16}/><span>Library</span></button><span className="breadcrumb-sep">/</span><span className="doc-crumb">{title || 'Untitled workspace'}</span></div><div className="editor-right"><span className="save-indicator"><span className={`save-dot ${saveState !== 'Saved' ? 'saving' : ''}`}/>{saveState} {saveState === 'Saved' && <span className="saved-time">· {new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' }).format(new Date(savedAt))}</span>}</span><div className="more-wrap"><button className="icon-button" onClick={() => setMenu(!menu)}><MoreHorizontal size={19}/></button>{menu && <div className="popover menu-popover"><button onClick={() => { setMenu(false); setDeleteConfirm(true); }} className="danger-action"><Trash2 size={14}/> Delete workspace</button></div>}</div></div></header>
    <div className="document-scroll"><div className="document"><div className="doc-date">{prettyDate(workspace.created_at)} <span>·</span> Quickspace note</div><input className="title-input" value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Untitled workspace"/><div className="editor-toolbar"><div className="toolbar-group"><ToolButton title="Paragraph" active={editor?.isActive('paragraph')} onClick={() => editor?.chain().focus().setParagraph().run()}><Text size={16}/></ToolButton><ToolButton title="Heading" active={editor?.isActive('heading')} onClick={() => editor?.chain().focus().toggleHeading({ level: 2 }).run()}><span className="heading-icon">H</span></ToolButton><span className="tool-divider"/><ToolButton title="Bold" active={editor?.isActive('bold')} onClick={() => editor?.chain().focus().toggleBold().run()}><b>B</b></ToolButton><ToolButton title="Italic" active={editor?.isActive('italic')} onClick={() => editor?.chain().focus().toggleItalic().run()}><i>I</i></ToolButton><span className="tool-divider"/><ToolButton title="Bulleted list" active={editor?.isActive('bulletList')} onClick={() => editor?.chain().focus().toggleBulletList().run()}><List size={16}/></ToolButton><ToolButton title="Numbered list" active={editor?.isActive('orderedList')} onClick={() => editor?.chain().focus().toggleOrderedList().run()}><ListOrdered size={16}/></ToolButton><ToolButton title="Code block" active={editor?.isActive('codeBlock')} onClick={() => editor?.chain().focus().toggleCodeBlock().run()}><Code2 size={16}/></ToolButton><span className="tool-divider"/><ToolButton title="Add image" onClick={pickImages}><ImagePlus size={16}/></ToolButton><ToolButton title="Add link" active={editor?.isActive('link')} onClick={setLink}><Link2 size={15}/></ToolButton></div></div>{error && <div className="notice error-notice editor-notice"><span>{error}</span><button onClick={clearError}><X size={15}/></button></div>}<EditorContent editor={editor}/>{!editor?.getText() && !editor?.getJSON().content?.some((n) => n.type === 'image') && <p className="inline-hint">Start typing, or drag a screenshot here.</p>}</div></div>
    {deleteConfirm && <div className="modal-backdrop"><div className="confirm-modal"><div className="confirm-icon"><Trash2 size={18}/></div><h3>Delete this workspace?</h3><p>This permanently removes the workspace and its copies of imported images.</p><div className="modal-actions"><button className="secondary-button" onClick={() => setDeleteConfirm(false)}>Cancel</button><button className="danger-button" onClick={onDelete}>Delete workspace</button></div></div></div>}</main>;
}
function ToolButton({ children, title, active, onClick }) { return <button type="button" className={`tool-button ${active ? 'active' : ''}`} title={title} onClick={onClick}>{children}</button>; }

function Settings({ settings, onChoose, dark, setDark, onBack }) {
  return <main className="settings-shell"><header className="settings-topbar"><button className="back-button" onClick={onBack}><ArrowLeft size={16}/> Library</button><span>Settings</span></header><div className="settings-content"><p className="eyebrow">PREFERENCES</p><h1>Settings</h1><p className="subhead">Quickspace runs quietly in the background and keeps your notes on this Mac.</p><section className="setting-section"><div className="setting-heading"><div><h2>Screenshot folder</h2><p>Quickspace watches this folder for new screenshots.</p></div><FolderOpen size={18}/></div><div className="path-picker"><div className="path-text"><span className="pulse-dot"/>{settings.watch_dir || 'No folder selected'}</div><button className="secondary-button" onClick={onChoose}>Choose folder</button></div><p className="setting-note">New PNG, JPG, JPEG, and WebP images will appear in the capture popup. The default screenshot location on macOS is your Desktop.</p></section><section className="setting-section"><div className="setting-heading"><div><h2>Appearance</h2><p>Choose a comfortable reading theme.</p></div><Sun size={18}/></div><div className="theme-toggle"><button className={!dark ? 'selected' : ''} onClick={() => setDark(false)}><Sun size={15}/> Light</button><button className={dark ? 'selected' : ''} onClick={() => setDark(true)}><Moon size={15}/> Dark</button></div></section><section className="setting-section"><div className="setting-heading"><div><h2>Quick capture shortcut</h2><p>Open the floating capture window from anywhere.</p></div><Code2 size={18}/></div><div className="shortcut-row"><span>Open capture popup</span><kbd>⌥</kbd><kbd>⇧</kbd><kbd>S</kbd></div></section><div className="settings-foot"><span className="tiny-mark"><Archive size={13}/></span> Quickspace · Your data stays on your Mac</div></div></main>;
}

export default App;
