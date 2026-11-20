**FREE
// Stage 2, S2.176: XML-INTO(E): a document that is not well formed, a
// value that does not fit its subfield's type, and a good one, %ERROR
// and %STATUS after each. Captured on PUB400 (tools/golden/native176.txt
// n176_xmlintoe) for the native unit NXMLINE.
ctl-opt dftactgrp(*no);
dcl-ds cust qualified;
  id int(10);
  name varchar(20);
end-ds;
dcl-s doc varchar(200);
dcl-s line varchar(120);

doc = '<cust><id>7</id><name>Ann</cust>';
xml-into(e) cust %xml(doc);
line = %char(%error) + ' ' + %char(%status);
doc = '<cust><id>x</id><name>Bo</name></cust>';
xml-into(e) cust %xml(doc);
line += ' ' + %char(%error) + ' ' + %char(%status);
doc = '<cust><id>9</id><name>Cy</name></cust>';
xml-into(e) cust %xml(doc);
line += ' ' + %char(%error) + ' ' + %char(cust.id) + cust.name;
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
