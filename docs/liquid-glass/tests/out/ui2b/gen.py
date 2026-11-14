import json
V={
 "background":["#ffffff","#0a0a0a"],"foreground":["#0a0a0a","#fafafa"],"card":["#ffffff","#171717"],"popover":["#ffffff","#171717"],
 "primary":["#171717","#e5e5e5"],"primary-foreground":["#fafafa","#171717"],"primary-hover":["#171717cc","#e5e5e5cc"],
 "secondary":["#f5f5f5","#262626"],"secondary-hover":["#e8e8e8","#333333"],"muted":["#f5f5f5","#262626"],"muted-half":["#f5f5f580","#26262680"],
 "muted-foreground":["#5c5c5c","#a8a8a8"],"accent":["#f5f5f5","#262626"],"destructive":["#c40d1f","#ff6467"],
 "destructive-tint":["#c40d1f1a","#ff646733"],"destructive-tint-hover":["#c40d1f33","#ff64674d"],"border":["#e5e5e5","#ffffff1a"],
 "input":["#d4d4d4","#ffffff26"],"control":["#ffffff00","#ffffff0b"],"hairline":["#0a0a0a1a","#fafafa1a"],"ring":["#a1a1a1","#737373"],
 "ring-soft":["#a1a1a180","#73737380"],"scrim":["#00000033","#00000099"]}
vars={k:{"type":"color","value":[{"value":a},{"value":b,"theme":{"Mode":"Dark"}}]} for k,(a,b) in V.items()}
for k,v in [("radius-xs",4),("radius-sm",6),("radius-md",8),("radius-lg",10),("radius-xl",14)]: vars[k]={"type":"number","value":v}
vars["font"]={"type":"string","value":"Segoe UI Variable"}
json.dump({"description":"Set the standard component tokens (light and dark)","operations":[{"op":"set_variables","themes":{"Mode":["Light","Dark"]},"variables":vars}]},open("a1.json","w"))

btn={"type":"frame","id":"ui-button","name":"Button","reusable":True,"height":36,"padding":[0,10],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
  {"type":"text","id":"ui-button-label","name":"Label","content":"Save changes","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$primary-foreground"}]}
btni={"type":"frame","id":"ui-button-icon-leading","name":"Button, leading icon","reusable":True,"height":36,"padding":[0,10,0,8],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
  {"type":"icon","id":"ui-button-icon-leading-icon","name":"Icon","library":"lucide","icon":"plus","width":16,"height":16,"fill":"$primary-foreground"},
  {"type":"text","id":"ui-button-icon-leading-label","name":"Label","content":"New project","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$primary-foreground"}]}
json.dump({"description":"Add the button components to a UI components board","operations":[{"op":"insert","node":{"type":"frame","id":"ui-components","name":"UI components","x":-600,"y":0,"layout":"vertical","gap":32,"padding":32,"fill":"#e5e5e5","children":[btn,btni]}}]},open("a2.json","w"))

W=136
def t(id,content,w):
    return {"type":"text","id":id,"content":content,"textGrowth":"fixed-width","width":w,"fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}
states=["Rest","Hover","Focus","Disabled","Loading"]
variants=[
 ("primary","Primary","Save changes",{}, None,"$primary-foreground",{"fill":"$primary-hover"},None),
 ("secondary","Secondary","Export CSV",{"fill":"$secondary"},"$foreground","$foreground",{"fill":"$secondary-hover"},None),
 ("outline","Outline","Cancel",{"fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner"},"$foreground","$foreground",{"fill":"$muted"},"$background"),
 ("ghost","Ghost","Skip for now",{"fill":"#00000000"},"$foreground","$foreground",{"fill":"$muted"},"$background"),
 ("destructive","Destructive","Delete project",{"fill":"$destructive-tint"},"$destructive","$destructive",{"fill":"$destructive-tint-hover"},["$background","$destructive-tint"]),
]
head={"type":"frame","id":"bs-head","name":"States","gap":24,"children":[{"type":"frame","id":"bs-head-0","width":88,"height":16}]+[t(f"bs-head-{i+1}",s,W) for i,s in enumerate(states)]}
rows=[head]
for key,name,copy,base,lab,labcol,hover,cover in variants:
    def inst(state,extra):
        d={"type":"ref","id":f"bs-{key}-{state}","name":f"Button / {key} / {state}","ref":"ui-button"}
        d.update(base); d.update(extra)
        ld={"content":copy}
        if lab: ld["fill"]=lab
        d["descendants"]={"ui-button-label":ld}
        return d
    def cell(state,node,ring=False):
        inner={"type":"frame","id":f"bs-{key}-{state}-pad","name":"Ring" if ring else "Pad","padding":3,"children":[node]}
        if ring: inner.update({"cornerRadius":"$radius-lg","fill":"$ring-soft"})
        return {"type":"frame","id":f"bs-{key}-{state}-cell","name":"Cell","width":W,"children":[inner]}
    focus={"stroke":"$ring","strokeWidth":1,"strokeAlignment":"inner"}
    if cover: focus["fill"]=cover
    load={"type":"ref","id":f"bs-{key}-loading","name":f"Button / {key} / loading","ref":"ui-button-icon-leading","opacity":0.5}
    load.update(base)
    load["descendants"]={"ui-button-icon-leading-icon":{"icon":"loader-circle","fill":labcol},"ui-button-icon-leading-label":{"content":copy,"fill":labcol}}
    rows.append({"type":"frame","id":f"bs-row-{key}","name":name,"gap":24,"alignItems":"center","children":[
        t(f"bs-row-{key}-name",name,88),
        cell("rest",inst("rest",{})),cell("hover",inst("hover",hover)),cell("focus",inst("focus",focus),True),
        cell("disabled",inst("disabled",{"opacity":0.5})),cell("loading",load)]})
board={"type":"frame","id":"button-states","name":"Button states","x":0,"y":0,"layout":"vertical","gap":16,"padding":32,"fill":"$background","children":rows}
json.dump({"description":"Build the button state board (light)","operations":[{"op":"insert","node":board}]},open("a3.json","w"))
