import hashlib,json,sys
from pathlib import Path
import polyxml
import polyxml._polyxml as native
expected=sys.argv[1]
fixture='/home/bailey/github/polyxml-xsd-attributes/research/fixtures/schema_attribute_entities.xsd'
xml=b'<Root><Status>R&amp;D</Status><Code>A&amp;B</Code><Fixed/><Default/></Root>'
try:
    result=polyxml.xml_to_json(xml,schema_path=fixture,root='Root')
    output={'result':'accepted','json':result.decode()}
except ValueError as error:
    output={'result':'rejected','error':str(error)}
output['module']=native.__file__
output['sha256']=hashlib.sha256(Path(native.__file__).read_bytes()).hexdigest()
assert output['result']==expected, output
print(json.dumps(output,indent=2))
