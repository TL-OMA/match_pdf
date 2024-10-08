MatchPDF
========

MatchPDF is a fast, command-line PDF comparison utility for Windows, optionally creating a new PDF document to illustrate the differences. 

MatchPDF runs locally, so the data never leaves your system.


Quick Start
-----------

    c:\> match_pdf.exe firstDoc.pdf secondDoc.pdf
    The PDF documents match.    



Flags
-----

    --stop or -s
Stop at the page where the first difference was detected.
<br/><br/>

    --maxpages ## or -m ##
At a maximum, compare ## pages. 
<br/><br/>

    --pages ## or -p ##
Stop after ## pages if there are differences in the first ## pages.
Note: The comparison will still stop at the first page with a difference if the ‘stop’ flag is also used.
<br/><br/>

    --output differences.pdf or -o differences.pdf  
Create a PDF file illustrating the differences side-by-side.
If other flags were used to limit the pages compared, this file will only contain those pages.
<br/><br/>

    --result result.json or -r result.json
Create a text file in JSON format showing whether the files match or differences were found.
<br/><br/>

    --justdiff or -j  
Only the pages with differences will be included in the output file.
Note: This is only effective if the ‘output’ argument is used.
<br/><br/>

    --config config.json or -c config.json 
Use a configuration file to exclude regions of the PDF.
<br/><br/>

    --debug or -d
Include verbose log information to the console to help troubleshoot issues.

<br/><br/>

For more details, see the User Guide that is part of the install.
