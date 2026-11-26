"""Studio-authored app appearance. Stable IDs never replace app/action identities."""
import json

# ------------------------=
# FUNC: load
# DESC: Reads a bounded installed appearance profile; missing profiles preserve factory layouts.
# ------------------=
def load(app_id, path='/System/app-design.json'):
    try:
        with open(path, 'r') as stream:
            raw = stream.read(65537)
        if len(raw) > 65536:
            return None
        document = json.loads(raw)
        if document.get('version') != 1:
            return None
        matches = [item for item in document.get('apps', []) if item.get('id') == app_id]
        if len(matches) != 1:
            return None
        item = matches[0]
        if item.get('enabled') is not True:
            return None
        if (not isinstance(item.get('title'), str) or not item['title'].strip()
                or len(item['title'].encode('utf-8')) > 96
                or any(ord(c) < 32 for c in item['title'])
                or item.get('menuAlignment') not in ('leading', 'trailing')
                or item.get('inspectorSide') not in ('left', 'right')
                or type(item.get('contentInset')) is not int
                or type(item.get('inspectorWidth')) is not int
                or not 8 <= item.get('contentInset', -1) <= 48
                or not 200 <= item.get('inspectorWidth', -1) <= 400):
            return None
        return item
    except (OSError, ValueError, TypeError, AttributeError):
        return None

# ------------------------=
# FUNC: apply
# DESC: Applies layout to real widgets so geometry managers also update their hit targets.
# ------------------=
def apply(root, app_id, menu=None, content=None, inspector=None, path='/System/app-design.json'):
    profile = load(app_id, path)
    if profile is None:
        return False
    root.title(profile['title'])
    if menu is not None:
        children = [(child, dict(child.pack_info())) for child in menu.pack_slaves()]
        # Preserve utility menus already anchored to the trailing edge (for example Help).
        for child, options in children:
            child.pack_forget()
        trailing = profile['menuAlignment'] == 'trailing'
        anchored = [(child, options) for child, options in children if options.get('side') == 'right']
        primary = [(child, options) for child, options in children if options.get('side') != 'right']
        for child, options in anchored:
            options.pop('in', None)
            child.pack(**options)
        for child, options in (reversed(primary) if trailing else primary):
            options.pop('in', None)
            options['side'] = 'right' if trailing else 'left'
            child.pack(**options)
    if content is not None:
        content.pack_configure(padx=profile['contentInset'], pady=profile['contentInset'])
    if inspector is not None:
        inspector.configure(width=profile['inspectorWidth'])
        inspector.pack_configure(side=profile['inspectorSide'])
    return True
