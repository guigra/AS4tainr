**FREE
// Stage 2, S2.200: a program call through a prototype whose EXTPGM names
// a variable: QDCXLATE by name in a field, translating a character field
// to ASCII (QTCPASC) and back (QTCPEBC); then the name in lower case.
// Captured on PUB400 (tools/golden/native200.txt n200_pgmvar) for the
// native unit NPGMVAR.
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
