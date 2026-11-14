import json
P="docs/liquid-glass/tests/photos/"
OUT="docs/liquid-glass/tests/out/4/"
UI="Segoe UI Variable"; DISP="Sitka"
def img(n): return {"type":"image","url":P+n+".jpg","mode":"cover"}
def t(c,s,w="400",fill="$on-dark-glass",**k):
    d={"type":"text","content":c,"fontFamily":UI,"fontSize":s,"fontWeight":w,"fill":fill}; d.update(k); return d
def srow(i,icon,label,count=None,sel=False):
    ch=[{"type":"icon","id":f"sb-row-{i}-icon","library":"lucide","icon":icon,"width":16,"height":16,"fill":"$on-dark-glass"},
        t(label,13,"600" if sel else "400",id=f"sb-row-{i}-label",width="fill_container",textGrowth="fixed-width")]
    if count: ch.append(t(count,12,"400",id=f"sb-row-{i}-count",textAlign="right"))
    d={"type":"frame","id":f"sb-row-{i}","name":label,"width":"fill_container","height":32,"cornerRadius":8,"padding":[0,12],"gap":12,"layout":"horizontal","alignItems":"center","children":ch}
    if sel: d["fill"]="$selection-on-dark"
    return d
def stitle(id,c): return {"type":"frame","id":id,"width":"fill_container","padding":[12,12,4,12],"children":[t(c,11,"600",opacity=0.65)]}
sidebar={"type":"frame","id":"sidebar","name":"Sidebar","x":8,"y":8,"width":248,"height":784,"cornerRadius":16,"padding":[16,8],"gap":4,"layout":"vertical",
 "material":{"type":"liquid_glass","appearance":"dark","depth":"panel","advanced":{"tintOpacity":0.8}},"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"win-dots","name":"Window controls","height":20,"padding":[0,12],"gap":8,"layout":"horizontal","alignItems":"center","children":[
    {"type":"ellipse","width":12,"height":12,"fill":"#ffffff47"},{"type":"ellipse","width":12,"height":12,"fill":"#ffffff47"},{"type":"ellipse","width":12,"height":12,"fill":"#ffffff47"}]},
  stitle("sb-t1","Library"),
  srow(1,"images","All photos","2,481"),srow(2,"heart","Favourites","126"),srow(3,"clock","Recently added","43"),srow(4,"map-pin","Places"),
  stitle("sb-t2","Albums"),
  srow(5,"sunset","Harbour at dusk","34",True),srow(6,"store","Saturday market","58"),srow(7,"ship","Night ferry","17"),srow(8,"trees","Forest light","212"),srow(9,"palette","Studio wall","9"),
  {"type":"frame","id":"sb-spacer","width":"fill_container","height":"fill_container"},
  {"type":"frame","id":"sb-storage","name":"Storage","width":"fill_container","padding":[8,12],"gap":8,"layout":"vertical","children":[
    {"type":"frame","id":"sb-storage-bar","width":"fill_container","height":4,"cornerRadius":2,"fill":"#ffffff2e","layout":"none","children":[
      {"type":"rectangle","x":0,"y":0,"width":166,"height":4,"cornerRadius":2,"fill":"$on-dark-glass"}]},
    t("41.2 GB of 50 GB used",12,"400")]}]}
def mrow(i,label,checked=False):
    lead={"type":"icon","library":"lucide","icon":"check","width":16,"height":16,"fill":"$on-dark-glass"} if checked else {"type":"frame","width":16,"height":16}
    return {"type":"frame","id":f"menu-row-{i}","width":"fill_container","height":32,"cornerRadius":8,"padding":[0,8],"gap":8,"layout":"horizontal","alignItems":"center","children":[lead,t(label,13,"400")]}
menu={"type":"frame","id":"sort-menu","name":"Sort menu","x":1006,"y":60,"width":232,"cornerRadius":16,"padding":8,"layout":"vertical",
 "material":{"type":"liquid_glass","appearance":"dark","depth":"panel"},"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","width":"fill_container","padding":[4,8,4,8],"children":[t("Sort by",11,"600",opacity=0.65)]},
  mrow(1,"Date taken",True),mrow(2,"Date added"),mrow(3,"Title"),
  {"type":"frame","width":"fill_container","padding":[4,8],"children":[{"type":"rectangle","width":"fill_container","height":1,"fill":"#ffffff29"}]},
  mrow(4,"Newest first",True),mrow(5,"Group by day"),mrow(6,"Favourites only")]}
G={"material":"liquid_glass","stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner"}
toolbar={"type":"frame","id":"toolbar","name":"Toolbar","x":778,"y":16,"layout":"horizontal","gap":8,"alignItems":"center","glassGroup":8,"children":[
  dict({"type":"frame","id":"search","name":"Search field","width":220,"height":36,"cornerRadius":18,"padding":[0,12],"gap":8,"layout":"horizontal","alignItems":"center","children":[
    {"type":"icon","library":"lucide","icon":"search","width":16,"height":16,"fill":"$on-dark-glass"},t("Search 2,481 photos",13,"400")]},**G),
  dict({"type":"frame","id":"sort-button","name":"Sort and filter","width":36,"height":36,"cornerRadius":18,"justifyContent":"center","alignItems":"center","children":[
    {"type":"icon","library":"lucide","icon":"arrow-up-down","width":16,"height":16,"fill":"$on-dark-glass"}]},**G),
  dict({"type":"frame","id":"actions","name":"Actions","width":118,"height":36,"cornerRadius":18,"padding":[0,16],"gap":16,"layout":"horizontal","alignItems":"center","children":[
    {"type":"icon","library":"lucide","icon":"heart","width":18,"height":18,"fill":"$on-dark-glass"},
    {"type":"icon","library":"lucide","icon":"share","width":18,"height":18,"fill":"$on-dark-glass"},
    {"type":"icon","library":"lucide","icon":"trash-2","width":18,"height":18,"fill":"$on-dark-glass"}]},**G)]}
imp={"type":"frame","id":"import","name":"Import","x":1176,"y":16,"width":88,"height":36,"cornerRadius":18,"justifyContent":"center","alignItems":"center","material":{"type":"liquid_glass","tint":"$accent"},
  "children":[t("Import",13,"600","#ffffff")]}
panel={"type":"frame","id":"album-panel","name":"Album details","x":288,"y":214,"width":400,"cornerRadius":24,"padding":24,"gap":12,"layout":"vertical","material":{"type":"frosted","thickness":"thick","appearance":"dark"},"children":[
  {"type":"text","content":"Harbour at dusk","fontFamily":DISP,"fontSize":32,"fontWeight":"600","lineHeight":1.1,"letterSpacing":-0.4,"fill":"#ffffff"},
  t("34 photos  ·  12 to 14 November 2026  ·  Shared with Ines and Tomasz",13,"400","#ffffff",width="fill_container",textGrowth="fixed-width"),
  t("Three evenings on the east quay, waiting for the 18:40 ferry to clear the breakwater.",15,"400","#ffffff",lineHeight=1.45,width="fill_container",textGrowth="fixed-width"),
  {"type":"frame","layout":"horizontal","gap":8,"padding":[8,0,0,0],"children":[
    {"type":"frame","name":"Play slideshow","height":36,"cornerRadius":18,"padding":[0,16],"gap":8,"alignItems":"center","fill":"#ffffff","children":[
      {"type":"icon","library":"lucide","icon":"play","width":14,"height":14,"fill":"$bg"},t("Play slideshow",13,"600","$bg")]},
    {"type":"frame","name":"Add photos","height":36,"cornerRadius":18,"padding":[0,16],"alignItems":"center","fill":"#ffffff29","children":[t("Add photos",13,"600","#ffffff")]}]}]}
def tile(x,w,n,name=None,count=None):
    out=[{"type":"rectangle","name":n,"x":x,"y":508,"width":w,"height":240,"fill":img(n)}]
    if name:
        out.append(t(name,13,"600","$text",x=x,y=758)); out.append(t(count,13,"400","$text-quiet",x=x+w-136,y=758,width=120,textAlign="right",textGrowth="fixed-width"))
    return out
window={"type":"frame","id":"window","name":"Photo library","x":0,"y":0,"width":1280,"height":800,"cornerRadius":24,"clip":True,"layout":"none","fill":"$bg","children":[
  {"type":"rectangle","id":"hero","name":"Hero photo","x":0,"y":0,"width":1280,"height":460,"fill":img("dusk-harbour")},
  t("More from this week",15,"600","$text",x=288,y=476),
  *tile(0,284,"night-ferry"),*tile(288,328,"market-colours","Saturday market","58 photos"),*tile(620,328,"bright-artwork","Studio wall","9 photos"),*tile(952,328,"forest-light","Forest light","212 photos"),
  panel,sidebar,toolbar,imp,menu]}
ops=[{"op":"set_variables","themes":{"Glass":["Clear","Tinted","Reduced transparency","Increased contrast"]},"variables":{
  "bg":{"type":"color","value":"#0d1017"},"text":{"type":"color","value":"#f3f1ee"},"text-quiet":{"type":"color","value":"#aaa7a2"},
  "on-light-glass":{"type":"color","value":"#000000"},"on-dark-glass":{"type":"color","value":"#ffffff"},
  "selection-on-light":{"type":"color","value":"#0000001f"},"selection-on-dark":{"type":"color","value":"#ffffff33"},
  "accent":{"type":"color","value":"#2347c5"},
  "glass-outline":{"type":"color","value":[{"value":"#00000000"},{"value":"#000000","theme":{"Glass":"Increased contrast"}}]},
  "glass-regular-tint-opacity":{"type":"number","value":[{"value":0.6,"theme":{"Glass":"Tinted"}},{"value":0.9,"theme":{"Glass":"Reduced transparency"}},{"value":1,"theme":{"Glass":"Increased contrast"}}]},
  "glass-regular-blur":{"type":"number","value":[{"value":40,"theme":{"Glass":"Reduced transparency"}},{"value":40,"theme":{"Glass":"Increased contrast"}}]},
  "glass-regular-refraction":{"type":"number","value":[{"value":0.4,"theme":{"Glass":"Reduced transparency"}},{"value":0,"theme":{"Glass":"Increased contrast"}}]}}},
 {"op":"delete","id":"window"},{"op":"insert","node":window}]
json.dump({"description":"Photo library window with glass sidebar, toolbar, menu and frosted album panel","operations":ops},open(OUT+"args.json","w"))
