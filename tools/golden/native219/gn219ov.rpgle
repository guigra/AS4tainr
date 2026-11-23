**FREE
// Interpreter fix (fork branch fix/overload-no-end-pr): an OVERLOAD
// prototype followed directly by a standalone field and the
// calculations, with no END-PR after it. Captured on PUB400
// (tools/golden/native219.txt n219_overload).
ctl-opt dftactgrp(*no);
dcl-pr fmt varchar(30) overload(fmtNum : fmtChr);
dcl-s line varchar(200);
dcl-s c char(5) inz('abc');

line = fmt(12.5) + '|' + fmt(c);
say(line);
*inlr = *on;

dcl-proc fmtNum;
  dcl-pi *n varchar(30);
    n packed(9:2) const;
  end-pi;
  return 'N' + %char(n);
end-proc;

dcl-proc fmtChr;
  dcl-pi *n varchar(30);
    s varchar(20) const;
  end-pi;
  return 'C' + %trim(s);
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
