import json
def c(l,d): return {"type":"color","value":[{"value":l},{"value":d,"theme":{"Mode":"Dark"}}]}
V={
"background":c("#ffffff","#0a0a0a"),"foreground":c("#0a0a0a","#fafafa"),"card":c("#ffffff","#171717"),"popover":c("#ffffff","#171717"),
"primary":c("#171717","#e5e5e5"),"primary-foreground":c("#fafafa","#171717"),"primary-hover":c("#171717cc","#e5e5e5cc"),
"secondary":c("#f5f5f5","#262626"),"muted":c("#f5f5f5","#262626"),"muted-half":c("#f5f5f580","#26262680"),
"muted-foreground":c("#5c5c5c","#a8a8a8"),"accent":c("#f5f5f5","#262626"),"destructive":c("#c40d1f","#ff6467"),
"destructive-tint":c("#c40d1f1a","#ff646733"),"border":c("#e5e5e5","#ffffff1a"),"input":c("#d4d4d4","#ffffff26"),
"control":c("#ffffff00","#ffffff0b"),"hairline":c("#0a0a0a1a","#fafafa1a"),"ring":c("#a1a1a1","#737373"),
"ring-soft":c("#a1a1a180","#73737380"),"scrim":{"type":"color","value":"#00000033"},
"radius-xs":{"type":"number","value":4},"radius-sm":{"type":"number","value":6},"radius-md":{"type":"number","value":8},
"radius-lg":{"type":"number","value":10},"radius-xl":{"type":"number","value":14},"font":{"type":"string","value":"Segoe UI"},
# added: the skill gives no hover for secondary and destructive
"secondary-hover":c("#ebebeb","#333333"),"destructive-tint-hover":c("#c40d1f33","#ff64674d"),
}
btn={"type":"frame","id":"ui-button","name":"Button","reusable":True,"height":36,"padding":[0,10],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
  {"type":"text","id":"ui-button-label","name":"Label","content":"Save changes","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$primary-foreground"}]}
btni={"type":"frame","id":"ui-button-icon-leading","name":"Button, leading icon","reusable":True,"height":36,"padding":[0,10,0,8],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
  {"type":"icon","id":"ui-button-icon-leading-icon","name":"Icon","library":"lucide","icon":"plus","width":16,"height":16,"fill":"$primary-foreground"},
  {"type":"text","id":"ui-button-icon-leading-label","name":"Label","content":"New project","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$primary-foreground"}]}
kit={"type":"frame","id":"ui-components","name":"UI components","x":-360,"y":0,"layout":"vertical","gap":32,"padding":32,"fill":"#e5e5e5","children":[btn,btni]}
OUT={"op":"#00000000"}
variants=[
 ("primary","Save changes",{}, "$primary-foreground", {"fill":"$primary-hover"}),
 ("secondary","Duplicate",{"fill":"$secondary"},"$foreground",{"fill":"$secondary-hover"}),
 ("outline","Cancel",{"fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner"},"$foreground",{"fill":"$muted"}),
 ("ghost","Skip for now",{"fill":"#00000000"},"$foreground",{"fill":"$muted"}),
 ("destructive","Delete project",{"fill":"$destructive-tint"},"$destructive",{"fill":"$destructive-tint-hover"}),
]
def inst(v,state):
    name,label,root,fg,hover=v
    n={"type":"ref","id":f"bs-{name}-{state}","name":f"Button / {name} / {state}"}
    n.update(root)
    if state=="loading":
        n["ref"]="ui-button-icon-leading"; n["opacity"]=0.5
        n["descendants"]={"ui-button-icon-leading-icon":{"icon":"loader-circle","fill":fg},"ui-button-icon-leading-label":{"content":label,"fill":fg}}
        return n
    n["ref"]="ui-button"
    n["descendants"]={"ui-button-label":{"content":label,"fill":fg}}
    if state=="hover": n.update(hover)
    if state=="disabled": n["opacity"]=0.5
    return n
def col(state,title):
    return {"type":"frame","id":f"bs-col-{state}","name":title,"layout":"vertical","gap":16,"children":[
      {"type":"text","id":f"bs-col-{state}-label","name":"Column label","content":title,"fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]+[inst(v,state) for v in variants]}
board={"type":"frame","id":"bs-light","name":"Button states","x":0,"y":0,"layout":"horizontal","gap":24,"padding":32,"fill":"$background","children":[
  col("rest","Rest"),col("hover","Hover"),col("disabled","Disabled"),col("loading","Loading")]}
json.dump({"description":"Tokens for the standard component set","operations":[{"op":"set_variables","themes":{"Mode":["Light","Dark"]},"variables":V}]},open("a1.json","w"))
json.dump({"description":"Button components and the state board without focus","operations":[{"op":"insert","node":kit},{"op":"insert","node":board}]},open("a2.json","w"))
