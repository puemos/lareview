import re

pr_ref = "puemos/hls-downloader/490"
pattern = r"^(?:https://github\.com/)?([^/]+)/([^/]+)/pull/(\d+)$|(?:([^/]+)/([^/#]+))?#?(\d+)$"
re_obj = re.compile(pattern)
match = re_obj.search(pr_ref)

if match:
    print(f"Match found!")
    for i, group in enumerate(match.groups()):
        print(f"Group {i+1}: {group}")
else:
    print("No match found")
