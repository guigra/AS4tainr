**FREE
// Stage 2, S2.165: XML-INTO from a document in a variable: into a data
// structure, and into a data structure array with the path and case
// options. Captured on PUB400 (tools/golden/native165.txt n165_xmlinto)
// for the native unit NXMLIN.
ctl-opt dftactgrp(*no);
dcl-ds cust qualified;
  id int(10);
  name varchar(20);
end-ds;
dcl-ds item qualified dim(3);
  code char(3);
  qty packed(5:0);
end-ds;
dcl-s doc varchar(200);
dcl-s line varchar(120);

doc = '<cust><id>7</id><name>Ann</name></cust>';
xml-into cust %xml(doc);
line = %char(cust.id) + ' ' + cust.name;
doc = '<order><List><Item><code>A1</code><qty>3</qty></Item>'
    + '<Item><code>B2</code><qty>12</qty></Item></List></order>';
xml-into item %xml(doc : 'path=order/list/item case=any');
line += ' ' + %trim(item(1).code) + '=' + %char(item(1).qty)
      + ' ' + %trim(item(2).code) + '=' + %char(item(2).qty)
      + ' [' + item(3).code + ']';
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
