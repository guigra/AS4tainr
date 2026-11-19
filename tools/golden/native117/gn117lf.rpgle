**FREE
// Stage 2, S2.117: procedures with files of their own. COUNT opens its
// file at each call and counts the records (twice: from the start each
// time); NEXT has a STATIC file that stays open, so each call reads the
// next record. GN117F is made by the capture (three records). Captured
// on PUB400 (tools/golden/native117.txt n117_localfile) for the native
// unit NLOCFIL.
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);
line = %char(count()) + ' ' + %char(count()) + ' ' + %char(next())
       + %char(next()) + %char(next());
say(line);
*inlr = *on;

dcl-proc count;
  dcl-pi *n int(10);
  end-pi;
  dcl-f gn117f disk usage(*input) qualified extfile('GUIGRA261/GN117F');
  dcl-ds rec likerec(gn117f.gn117f);
  dcl-s k int(10);
  read gn117f rec;
  dow not %eof(gn117f);
    k += 1;
    read gn117f rec;
  enddo;
  return k;
end-proc;

dcl-proc next;
  dcl-pi *n int(10);
  end-pi;
  dcl-f gn117f disk usage(*input) static qualified
                 extfile('GUIGRA261/GN117F');
  dcl-ds rec likerec(gn117f.gn117f);
  read gn117f rec;
  return rec.n;
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
