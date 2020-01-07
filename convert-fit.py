#!/usr/bin/env python
from __future__ import print_function

import fitparse
import codecs
import datetime
import os
import json
import types

input_dir = 'data'
output_dir = 'data-out'

class RecordJSONEncoder(json.JSONEncoder):
    def default(self, obj):
        if isinstance(obj, types.GeneratorType):
            return list(obj)
        if isinstance(obj, datetime.datetime):
            return obj.isoformat()
        if isinstance(obj, fitparse.DataMessage):
            return {
                "type": obj.name,
                "data": {
                    data.name: data.value for data in obj
                }
            }
        return super(RecordJSONEncoder, self).default(obj)

for fname in os.listdir(input_dir):
    if fname.endswith(".fit"):
        name, ext = str.split(fname, '.')
        output_file = output_dir + '/' + name + '.json'
        if os.path.exists(output_file):
            continue
        
        try:
            fitfile = fitparse.FitFile(
                input_dir + '/' + fname,
                data_processor=fitparse.StandardUnitsDataProcessor(),
                check_crc=False
            )
            
            records = fitfile.get_messages(
                name='session'
            )
            
            fp = codecs.open(output_file, 'w', encoding='UTF-8')
            json.dump(records, fp=fp, cls=RecordJSONEncoder)
        except:
            print("Error reading " + fname);
