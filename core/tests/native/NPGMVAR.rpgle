**FREE
// Native unit NPGMVAR (S2.200): EXTPGM naming a variable, the program
// captured on PUB400 as GN200PV with DSPLY for its output.
// Source: golden capture tools/golden/native200 n200_pgmvar (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-s pname char(10) inz('QDCXLATE');
dcl-pr xlate extpgm(pname);
  len packed(5:0) const;
  data char(5);
  table char(10) const;
end-pr;
dcl-s data char(5) inz('hello');
dcl-s line varchar(200);

callp(e) xlate(5 : data : 'QTCPASC');
line = %char(data <> 'hello') + ' ' + %char(%status);
callp(e) xlate(5 : data : 'QTCPEBC');
line += ' ' + data + ' ' + %char(%status);
pname = 'qdcxlate';
callp(e) xlate(5 : data : 'QTCPASC');
line += ' ' + data + ' ' + %char(%status);
dsply (line);
*inlr = *on;
