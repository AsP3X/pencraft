import json
UI="Segoe UI Variable"
def th(l,d): return [{"value":l},{"value":d,"theme":{"Mode":"Dark"}}]
vars_={
 "bg":{"type":"color","value":th("#f2f4ef","#101614")},
 "bg-fade":{"type":"color","value":th("#f2f4efe6","#101614e6")},
 "bg-clear":{"type":"color","value":th("#f2f4ef00","#10161400")},
 "text":{"type":"color","value":th("#16201c","#e8eee9")},
 "quiet":{"type":"color","value":th("#56635d","#9daaa3")},
 "line":{"type":"color","value":th("#d3d9d2","#27322e")},
 "accent-text":{"type":"color","value":th("#0a6b55","#62d1b1")},
 "on-light-glass":{"type":"color","value":"#000000"},
 "on-dark-glass":{"type":"color","value":"#ffffff"},
 "selection-on-light":{"type":"color","value":"#0000001f"},
 "selection-on-dark":{"type":"color","value":"#ffffff33"},
 "accent":{"type":"color","value":"#0a6b55"},
 "glass-outline":{"type":"color","value":[{"value":"#00000000"},{"value":"#000000","theme":{"Glass":"Increased contrast"}}]},
 "glass-regular-tint-opacity":{"type":"number","value":[{"value":0.6,"theme":{"Glass":"Tinted"}},{"value":0.9,"theme":{"Glass":"Reduced transparency"}},{"value":1,"theme":{"Glass":"Increased contrast"}}]},
 "glass-regular-blur":{"type":"number","value":[{"value":40,"theme":{"Glass":"Reduced transparency"}},{"value":40,"theme":{"Glass":"Increased contrast"}}]},
 "glass-regular-refraction":{"type":"number","value":[{"value":0.4,"theme":{"Glass":"Reduced transparency"}},{"value":0,"theme":{"Glass":"Increased contrast"}}]},
}
def tab(n,icon,label,sel):
    c="$accent" if sel else "$on-light-glass"
    f={"type":"frame","id":f"glass-tab-{n}","width":88,"height":54,"cornerRadius":27,"layout":"vertical","gap":2,"justifyContent":"center","alignItems":"center","children":[
      {"type":"icon","id":f"glass-tab-{n}-icon","icon":icon,"width":22,"height":22,"fill":c},
      {"type":"text","id":f"glass-tab-{n}-label","content":label,"fontFamily":UI,"fontSize":11,"fontWeight":"600","fill":c}]}
    if sel: f["fill"]="$selection-on-light"
    return f
G={"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner"}
tabbar={"type":"frame","id":"glass-tab-bar","name":"Tab bar","reusable":True,"layout":"horizontal","gap":10,"alignItems":"center","glassGroup":10,"children":[
  {"type":"frame","id":"glass-tabs","name":"Tabs","width":279,"height":62,"cornerRadius":31,"padding":4,"layout":"horizontal","justifyContent":"space_between","alignItems":"center","material":{"type":"liquid_glass","depth":"bar"},**G,"children":[
    tab(1,"newspaper","Today",True),tab(2,"library","Library",False),tab(3,"bookmark","Saved",False)]},
  {"type":"frame","id":"glass-tab-search","name":"Search tab","width":62,"height":62,"cornerRadius":31,"justifyContent":"center","alignItems":"center","material":{"type":"liquid_glass","depth":"bar"},**G,"children":[
    {"type":"icon","id":"glass-tab-search-icon","icon":"search","width":24,"height":24,"fill":"$on-light-glass"}]}]}
toolbar={"type":"frame","id":"glass-toolbar","name":"Toolbar","reusable":True,"width":361,"layout":"horizontal","justifyContent":"space_between","alignItems":"center","children":[
  {"type":"frame","id":"glass-toolbar-back","name":"Back","width":44,"height":44,"cornerRadius":22,"justifyContent":"center","alignItems":"center","material":"liquid_glass",**G,"children":[
    {"type":"icon","id":"glass-toolbar-back-icon","icon":"chevron-left","width":22,"height":22,"fill":"$on-light-glass"}]},
  {"type":"frame","id":"glass-toolbar-trailing","layout":"horizontal","gap":10,"alignItems":"center","children":[
    {"type":"frame","id":"glass-toolbar-actions","name":"Actions","height":44,"cornerRadius":22,"padding":[0,14],"gap":18,"layout":"horizontal","alignItems":"center","material":"liquid_glass",**G,"children":[
      {"type":"icon","id":"glass-toolbar-action-1","icon":"a-large-small","width":22,"height":22,"fill":"$on-light-glass"},
      {"type":"icon","id":"glass-toolbar-action-2","icon":"list-filter","width":22,"height":22,"fill":"$on-light-glass"},
      {"type":"icon","id":"glass-toolbar-action-3","icon":"ellipsis","width":22,"height":22,"fill":"$on-light-glass"}]},
    {"type":"frame","id":"glass-toolbar-primary","name":"Listen","width":44,"height":44,"cornerRadius":22,"justifyContent":"center","alignItems":"center","material":{"type":"liquid_glass","tint":"$accent"},"children":[
      {"type":"icon","id":"glass-toolbar-primary-icon","icon":"headphones","width":22,"height":22,"fill":"#ffffff"}]}]}]}
board={"type":"frame","id":"glass-board","name":"Glass components","x":-600,"y":0,"layout":"vertical","gap":40,"padding":40,"fill":"#8a93a3","children":[tabbar,toolbar]}
ops=[{"op":"set_variables","themes":{"Mode":["Light","Dark"],"Glass":["Clear","Tinted","Reduced transparency","Increased contrast"]},"variables":vars_},
     {"op":"insert","node":board}]
json.dump({"description":"Tokens and glass components","operations":ops},open("out/2/step1.json","w"))
