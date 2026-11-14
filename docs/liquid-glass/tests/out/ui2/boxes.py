import json,sys
d=json.load(sys.stdin)
def w(n,ind=0):
    if isinstance(n,dict):
        if 'layoutBox' in n: print(' '*ind,n.get('id'),n.get('type'),n['layoutBox'])
        for c in n.get('children',[]): w(c,ind+2)
    elif isinstance(n,list):
        for c in n: w(c,ind)
w(d.get('layers',d))
