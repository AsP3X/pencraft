import json
def heading(id, text):
    return {"type":"frame","id":id,"name":"Group heading","width":"fill_container","height":28,"padding":[0,8],"alignItems":"center","children":[
      {"type":"text","id":id+"-text","content":text,"fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]}
def item(id, icon, label, kbd=None, selected=False):
    ch=[{"type":"icon","id":id+"-icon","library":"lucide","icon":icon,"width":16,"height":16,"fill":"$muted-foreground"},
        {"type":"text","id":id+"-label","content":label,"textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]
    if kbd:
        ch.append({"type":"frame","id":id+"-kbd","name":"Kbd","width":20,"height":20,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-xs","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
          {"type":"text","id":id+"-kbd-text","content":kbd,"fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"}]})
    f={"type":"frame","id":id,"name":"Item, selected" if selected else "Item","width":"fill_container","height":40,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","children":ch}
    if selected: f["fill"]="$muted"
    return f
opened={"type":"frame","id":"ui-command","name":"Command palette / opened","reusable":True,"width":560,"layout":"vertical","cornerRadius":"$radius-xl","fill":"$popover","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":[{"type":"shadow","offset":{"x":0,"y":2},"blur":6,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":16},"blur":48,"color":"#00000029"}],"children":[
  {"type":"frame","id":"ui-command-input","name":"Input","width":"fill_container","height":48,"padding":[0,16],"gap":8,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
    {"type":"icon","id":"ui-command-input-icon","library":"lucide","icon":"search","width":16,"height":16,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-command-input-value","name":"Value","content":"Type a command or search","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-command-list","name":"List","width":"fill_container","layout":"vertical","padding":8,"children":[
    heading("ui-command-heading","Suggestions"),
    item("ui-command-item-1","calendar","Open this week's rota","R",True),
    item("ui-command-item-2","arrow-left-right","Request a shift swap","S"),
    item("ui-command-item-3","plus","Add a shift","N"),
    heading("ui-command-heading-2","Go to"),
    item("ui-command-item-4","users","Team availability"),
    item("ui-command-item-5","calendar-off","Time off requests"),
    item("ui-command-item-6","settings","Workspace settings")]}]}
typed={"type":"ref","id":"cmd-typed","name":"Command palette / typed 'swap'","ref":"ui-command","descendants":{
  "ui-command-input-value":{"content":"swap","fill":"$foreground"},
  "ui-command-list":{"children":[
    heading("typed-heading-1","Commands"),
    item("typed-item-1","arrow-left-right","Request a shift swap","S",True),
    item("typed-item-2","circle-check","Approve swap requests"),
    heading("typed-heading-2","Open swaps"),
    item("typed-item-3","user","Maren Okafor wants to swap Thursday's early shift"),
    item("typed-item-4","user","Tobias Lindqvist offers Sat 21 Nov, 14:00 to 22:00")]}}}
empty={"type":"ref","id":"cmd-empty","name":"Command palette / no results","ref":"ui-command","descendants":{
  "ui-command-input-value":{"content":"holiday pay","fill":"$foreground"},
  "ui-command-list":{"padding":[24,16],"alignItems":"center","children":[
    {"type":"text","id":"empty-text","name":"Empty","content":"No results for holiday pay","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}}}
board={"type":"frame","id":"moments","name":"Command palette, three moments","x":0,"y":0,"layout":"horizontal","gap":48,"alignItems":"start","children":[opened,typed,empty]}
json.dump({"description":"Command palette in three moments: opened, typed 'swap', no results","operations":[{"op":"insert","node":board}]},open("build.json","w"))
