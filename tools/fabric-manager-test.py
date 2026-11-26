"""Behavioral host interaction checks for the production Fabric Manager controls."""
import json
from pathlib import Path
import runpy
import sys
import tkinter as tk
import types
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'sdk/python'))
resource = {'known':1, 'usable':1, 'capacity':4096, 'available':3072, 'provider_reserved':0, 'consumer_reserved':1024}
state = {'observed_at':10, 'totals':[2, 64*1024*1024, 0], 'nodes':[
    {'id':'01'*32,'local':True,'reachability':'Online','trust':'Trusted','last_seen':10,'resources':[dict(resource) for _ in range(3)]}],
    'provider':{'enabled':True,'revision':1,'memory':64*1024*1024,'cpu':2,'tasks':2,'active':0,'allocated_memory':0}}
writes=[]
jobs=[]
cancellations=[]
service_available=True

# ------------------------=
# FUNC: request
# DESC: Models revision-checked service state so UI actions can be checked independently of presentation text.
# ------------------=
def request(action,*args):
    if not service_available: raise OSError('service disconnected')
    if action==3:
        ident,epoch=args
        job=next(job for job in jobs if job['id']==ident)
        if epoch!=job['epoch']: raise OSError('stale epoch')
        cancellations.append(args)
        job['state']=6
        return b''
    if action==2: return json.dumps(jobs).encode()
    if action==0: return json.dumps(state).encode()
    revision,enabled,memory,packed=args
    if revision!=state['provider']['revision']: raise OSError('conflict')
    writes.append(args)
    state['provider'].update(revision=revision+1,enabled=bool(enabled),memory=memory,cpu=packed&65535,tasks=packed>>16)
    return b''

sys.modules['_infinity_app']=types.SimpleNamespace(fabric_request=request)
root=tk.Tk()
try:
    root.maxsize(900, 580)
    app=runpy.run_path('runtime/install/System/infinity-fabric.py')['FabricManager'](root)
    root.update()
    assert all(not icon.winfo_manager() for icon in app.card_icons)
    before=app.enabled.get()
    app.toggle.invoke(); root.update()
    assert app.enabled.get()!=before and app.dirty
    app.reload_policy()
    assert app.enabled.get()==state['provider']['enabled']
    assert root.minsize()[0] <= 900 and root.minsize()[1] <= 580
    assert root.winfo_width() <= 900 and root.winfo_height() <= 580
    node=state['nodes'][0]['id']
    assert app.nodes.bbox(node), 'First node must be visible at minimum display size'
    assert app.nodes.winfo_height() >= 80
    assert app.jobs.winfo_height() >= 60
    app.inspector_canvas.yview_moveto(1.0); root.update()
    assert app.inspector_canvas.yview()[1] > .99
    root.maxsize(1600, 1000)
    header_height=root.winfo_children()[0].winfo_height()
    assert header_height>=36
    root.geometry('1200x800');root.update()
    assert root.winfo_children()[0].winfo_height()==header_height
    assert all(icon.winfo_manager() for icon in app.card_icons)
    for icon in app.card_icons:
        assert icon.winfo_width()==44 and icon.winfo_height()==44
    jobs.append({'id':1,'epoch':1,'state':3,'cpu':2,'memory':1048576,'node':'01'*32})
    app.refresh()
    assert app.jobs.get_children() == ('1',)
    app.jobs.selection_set('1')
    app.select_job()
    assert not app.cancel_button.instate(['disabled'])
    jobs[0]['epoch']=2
    app.cancel_job()
    assert not cancellations and jobs[0]['state']==3
    app.refresh(); app.cancel_job()
    assert cancellations==[(1,2)] and jobs[0]['state']==6
    assert app.cancel_button.instate(['disabled'])
    jobs[0]['state']=4
    app.refresh()
    assert app.jobs.selection() == ('1',)
    jobs.clear(); app.refresh()
    assert not app.jobs.get_children()
    node=state['nodes'][0]['id']
    app.nodes.selection_set(node);app.select()
    assert app.selected_node['id']==node
    assert app.selected_node['resources'][0]['consumer_reserved']==1024
    app.cpu.set('4');app.mark_dirty();app.refresh()
    assert app.cpu.get()=='4'
    assert app.nodes.selection()==(node,)
    app.apply_button.invoke()
    assert len(writes)==1 and state['provider']['cpu']==4
    assert not app.dirty
    app.cpu.set('-1');app.mark_dirty();app.apply()
    assert len(writes)==1 and app.dirty
    app.cpu.set('5');state['provider']['revision']+=1;app.apply()
    assert len(writes)==1 and app.dirty and app.policy_error
    rejected_revision = app.edit_revision
    app.refresh(automatic=True)
    assert app.policy_error and app.edit_revision == rejected_revision
    assert app.notice is not None
    app.reload_policy()
    assert not app.policy_error
    assert not app.dirty and app.edit_revision==state['provider']['revision']
    state['nodes']=[];app.refresh()
    assert not app.nodes.get_children()
    assert app.selected_node is None
    state['provider']=None;app.refresh()
    assert app.apply_button.instate(['disabled'])
    before=app.enabled.get(); app.toggle.invoke()
    assert app.enabled.get()==before
    app.apply()
    assert len(writes)==1
    # Losing the authority must remove stale observations and disable mutations.
    state['provider']={'enabled':True,'revision':3,'memory':67108864,'cpu':4,'tasks':2,'active':1,'allocated_memory':1048576}
    jobs.append({'id':2,'epoch':1,'state':3,'cpu':2,'memory':1048576,'node':'01'*32})
    app.refresh()
    app.jobs.selection_set('2'); app.select_job()
    assert not app.cancel_button.instate(['disabled'])
    service_available=False
    app.refresh()
    assert app.snapshot is None and app.selected_node is None
    assert not app.job_records and not app.jobs.get_children()
    assert not app.nodes.get_children()
    assert app.cancel_button.instate(['disabled'])
    assert app.apply_button.instate(['disabled'])
    assert all(control.instate(['disabled']) for control in app.entries)
    service_available=True
    app.refresh()
    assert app.snapshot is not None and app.jobs.get_children()==('2',)
    assert not app.apply_button.instate(['disabled'])
finally:
    root.destroy()
