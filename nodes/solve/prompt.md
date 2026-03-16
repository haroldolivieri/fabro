Goal: HttpResponse doesn't handle memoryview objects
Description
	
I am trying to write a BinaryField retrieved from the database into a HttpResponse. When the database is Sqlite this works correctly, but Postgresql returns the contents of the field as a memoryview object and it seems like current Django doesn't like this combination:
from django.http import HttpResponse																	 
# String content
response = HttpResponse("My Content")																			
response.content																								 
# Out: b'My Content'
# This is correct
# Bytes content
response = HttpResponse(b"My Content")																		 
response.content																								 
# Out: b'My Content'
# This is also correct
# memoryview content
response = HttpResponse(memoryview(b"My Content"))															 
response.content
# Out: b'<memory at 0x7fcc47ab2648>'
# This is not correct, I am expecting b'My Content'



## Additional Context

I guess HttpResponseBase.make_bytes ​could be adapted to deal with memoryview objects by casting them to bytes. In all cases simply wrapping the memoryview in bytes works as a workaround HttpResponse(bytes(model.binary_field)).
The fact make_bytes would still use force_bytes if da56e1bac6449daef9aeab8d076d2594d9fd5b44 didn't refactor it and that d680a3f4477056c69629b0421db4bb254b8c69d0 added memoryview support to force_bytes strengthen my assumption that make_bytes should be adjusted as well.
I'll try to work on this.

## Completed stages
- **setup**: fail
  - Script: `git clone https://github.com/django/django.git . && git checkout 879cc3da6249e920b8d54518a0ae06de835d7373 && python -m pip install -e .`
  - Stdout:
    ```
    fatal: destination path '.' already exists and is not an empty directory.
    ```
  - Stderr: (empty)

## Context
- failure_class: deterministic
- failure_signature: setup|deterministic|script failed with exit code: <n> ## stdout fatal: destination path '.' already exists and is not an empty directory.


Fix this GitHub issue in the repository. Make the minimal code change needed.