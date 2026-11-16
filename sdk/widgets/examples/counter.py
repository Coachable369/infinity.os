import tkinter as tk
from tkinter import ttk

root = tk.Tk(className='InfinityWidget')
root.title('Infinity widget')
root.geometry('360x200')
root.configure(background='#090F26')
style = ttk.Style(root)
style.theme_use('clam')
style.configure('.', **{'background': '#101B3A', 'foreground': '#FFFFFF', 'font': ('Inter', 11), 'fieldbackground': '#090F26', 'bordercolor': '#86C7FF', 'lightcolor': '#1A2E63', 'darkcolor': '#090F26'})
style.map('TButton', background=[('active', '#1A2E63'), ('pressed', '#3E6FFF')])
controls = {}
from infinity_widget import attach
widget_host = attach(root, __file__)
controls['title'] = ttk.Label(root, text='Session counter')
controls['title'].place(relx=0.05555555555555555, relwidth=0.8333333333333334, y=16, height=32)
controls['count'] = ttk.Label(root, text='0')
controls['count'].place(relx=0.05555555555555555, relwidth=0.8333333333333334, y=58, height=42)
controls['add'] = ttk.Button(root, text='Add one')
controls['add'].place(relx=0.05555555555555555, relwidth=0.4111111111111111, y=126, height=40)
controls['reset'] = ttk.Button(root, text='Reset')
controls['reset'].place(relx=0.5111111111111111, relwidth=0.4111111111111111, y=126, height=40)

total = 0

# ------------------------=
# FUNC: increment
# DESC: Adds one to the displayed count.
# ------------------=
def increment():
    global total
    total += 1
    controls['count'].configure(text=str(total))

# ------------------------=
# FUNC: reset_count
# DESC: Resets the counter and its label together.
# ------------------=
def reset_count():
    global total
    total = 0
    controls['count'].configure(text='0')


controls['add'].configure(command=increment)
controls['reset'].configure(command=reset_count)
root.mainloop()
