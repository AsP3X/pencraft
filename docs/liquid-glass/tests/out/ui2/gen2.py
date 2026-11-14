import json
ring={"stroke":"$ring","strokeWidth":1,"strokeAlignment":"inner"}
variants=[
 ("primary","Save changes",{}, "$primary-foreground",104),
 ("secondary","Duplicate",{"fill":"$secondary"},"$foreground",79),
 ("outline","Cancel",{"fill":"$background"},"$foreground",61),
 ("ghost","Skip for now",{"fill":"$background"},"$foreground",98),
 ("destructive","Delete project",{"fill":["$background","$destructive-tint"]},"$destructive",107),
]
def slot(v):
    name,label,root,fg,w=v
    r={"type":"ref","id":f"bs-{name}-focus-control","name":"Button","ref":"ui-button","descendants":{"ui-button-label":{"content":label,"fill":fg}}}
    r.update(root); r.update(ring)
    return {"type":"frame","id":f"bs-{name}-focus","name":f"Button / {name} / focus","layout":"none","width":w,"height":36,"children":[
      {"type":"frame","id":f"bs-{name}-focus-ring","name":"Ring","x":-3,"y":-3,"padding":3,"cornerRadius":"$radius-lg","fill":"$ring-soft","children":[r]}]}
col={"type":"frame","id":"bs-col-focus","name":"Focus","layout":"vertical","gap":16,"children":[
  {"type":"text","id":"bs-col-focus-label","name":"Column label","content":"Focus","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]+[slot(v) for v in variants]}
json.dump({"description":"Focus column, and the dark board","operations":[
 {"op":"insert","parent":"bs-light","index":2,"node":col},
 {"op":"copy","id":"bs-light","properties":{"name":"Button states, dark","theme":{"Mode":"Dark"},"x":763,"y":0}}]},open("a3.json","w"))
