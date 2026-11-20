**FREE
// Native unit NXMLINE (S2.176): XML-INTO(E), the program captured on
// PUB400 as GN176XE with DSPLY for its output.
// Source: golden capture tools/golden/native176 n176_xmlintoe (PUB400,
// IBM i 7.5, 2026-11-20).
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
dsply (line);
*inlr = *on;
