import json
UI="Segoe UI Variable"; SERIF="Sitka"
P="docs/liquid-glass/tests/photos/"
arts=[
 ("Tideline Review","18 min","The harbour pilots who still navigate by memory","Before the radar screens, before the dredged channel, there were forty-one marks a pilot had to carry in his head. Three men on the estuary still do, and none of them has an apprentice.",None),
 ("Field Notes","9 min","What a beech wood does with the first hour of light","Foresters call it the green hour. For roughly fifty minutes after sunrise the canopy passes on only a sliver of the spectrum, and nearly everything living below has arranged its day around it.","forest-light.jpg"),
 ("The Ledger","24 min","A short history of the market stall, told through its awnings","Stripes meant fish, plain canvas meant cloth, and a trader who changed colour without asking the guild could expect a visit before noon.",None),
 ("Night Desk","12 min","Last crossing: a winter on the 23:40 ferry","The same nineteen passengers, a kettle that has outlived two captains, and a timetable nobody has needed to read since 1998.","night-ferry.jpg"),
 ("Marginalia","7 min","Why we underline: notes on reading with a pencil","A marked page is a conversation you can come back to. An unmarked one, the argument goes, is only a page you once looked at.",None),
 ("Tideline Review","31 min","The cartographer of things that are no longer there","She has mapped 212 demolished piers, each drawn at the scale of the one that replaced it, and keeps the sheets in the order the tide took them.","dusk-harbour.jpg"),
 ("Field Notes","11 min","Colour words a dyer needs and a painter does not","Madder has nine named reds in the old Norwich pattern books. Six of them describe the cloth while it is still wet.","market-colours.jpg"),
]
def art(s,i,a):
    src,mins,head,ex,img=a
    textcol={"type":"frame","layout":"vertical","gap":8,"width":"fill_container","children":[
        {"type":"frame","layout":"horizontal","width":"fill_container","justifyContent":"space_between","alignItems":"center","children":[
          {"type":"text","content":src,"fontFamily":UI,"fontSize":13,"fontWeight":"700","fill":"$accent-text"},
          {"type":"text","content":mins,"fontFamily":UI,"fontSize":13,"fontWeight":"400","fill":"$quiet"}]},
        {"type":"text","content":head,"fontFamily":SERIF,"fontSize":22,"fontWeight":"700","lineHeight":1.15,"letterSpacing":-0.2,"fill":"$text","width":"fill_container","textGrowth":"fixed-width"}]}
    top=textcol
    if img:
        top={"type":"frame","layout":"horizontal","gap":16,"alignItems":"start","width":"fill_container","children":[textcol,
           {"type":"rectangle","name":"Picture","width":84,"height":84,"cornerRadius":8,"fill":{"type":"image","url":P+img,"mode":"cover"}}]}
    ch=[top,{"type":"text","content":ex,"fontFamily":UI,"fontSize":15,"fontWeight":"400","lineHeight":1.45,"fill":"$quiet","width":"fill_container","textGrowth":"fixed-width"}]
    row={"type":"frame","id":f"{s}-art-{i}","name":head[:24],"layout":"vertical","gap":12,"width":"fill_container","padding":[20,0],"children":ch}
    return row
def screen(s,x,dark):
    feedch=[]
    for i,a in enumerate(arts):
        if i: feedch.append({"type":"rectangle","width":"fill_container","height":1,"fill":"$line"})
        feedch.append(art(s,i,a))
    on="$on-dark-glass" if dark else "$on-light-glass"
    tb={"type":"ref","id":f"{s}-toolbar","ref":"glass-toolbar","x":16,"y":58}
    tab={"type":"ref","id":f"{s}-tabbar","ref":"glass-tab-bar","x":21,"y":769}
    if dark:
        tb["descendants"]={k:{"fill":on} for k in ["glass-toolbar-back-icon","glass-toolbar-action-1","glass-toolbar-action-2","glass-toolbar-action-3"]}
        d={k:{"fill":on} for k in ["glass-tab-1-icon","glass-tab-1-label","glass-tab-2-icon","glass-tab-2-label","glass-tab-3-icon","glass-tab-3-label","glass-tab-search-icon"]}
        d["glass-tab-1"]={"fill":"$selection-on-dark"}
        tab["descendants"]=d
    def fade(y,rot,name): return {"type":"rectangle","id":s+"-"+name.split()[-1],"name":name,"x":0,"y":y,"width":393,"height":120,"fill":{"type":"gradient","gradientType":"linear","rotation":rot,"colors":[{"color":"$bg-clear","position":0},{"color":"$bg-fade","position":0.55},{"color":"$bg-solid","position":0.8 if rot==0 else 1}]}}
    f={"type":"frame","id":s,"name":"Reader, dark" if dark else "Reader, light","x":x,"y":0,"width":393,"height":852,"layout":"none","clip":True,"fill":"$bg","children":[
       {"type":"frame","id":f"{s}-feed","name":"Feed","x":0,"y":-212,"width":393,"layout":"vertical","padding":[0,20],"children":feedch},
       fade(0,0,"Scroll edge top"),fade(732,180,"Scroll edge bottom"),
       {"type":"frame","id":f"{s}-status","name":"Status bar","x":0,"y":14,"width":393,"height":24,"layout":"horizontal","padding":[0,32],"justifyContent":"space_between","alignItems":"center","children":[
          {"type":"text","content":"9:41","fontFamily":UI,"fontSize":16,"fontWeight":"600","fill":"$text"},
          {"type":"frame","layout":"horizontal","gap":8,"alignItems":"center","children":[
            {"type":"icon","icon":"signal","width":16,"height":16,"fill":"$text"},
            {"type":"icon","icon":"wifi","width":16,"height":16,"fill":"$text"},
            {"type":"icon","icon":"battery-medium","width":22,"height":22,"fill":"$text"}]}]},
       tb,tab]}
    if dark: f["theme"]={"Mode":"Dark"}
    return f
ops=[{"op":"delete","id":"light"},{"op":"delete","id":"dark"},{"op":"set_variables","variables":{"accent":{"type":"color","value":"#085a47"},"bg-solid":{"type":"color","value":[{"value":"#f2f4eff2"},{"value":"#101614f2","theme":{"Mode":"Dark"}}]},"bg-fade":{"type":"color","value":[{"value":"#f2f4ef99"},{"value":"#10161499","theme":{"Mode":"Dark"}}]}}},{"op":"insert","node":screen("light",0,False)},{"op":"insert","node":screen("dark",433,True)}]
json.dump({"description":"Two reader screens","operations":ops},open("out/2/step4.json","w"))
