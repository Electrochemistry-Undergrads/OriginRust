// Our own Origin C helper for the statistics reference runs (stats-anova.lines.txt, stats-norm.lines.txt).
// It reads the numeric columns A, B, D of the active sheet, calls Origin's public statistics library
// functions, and writes every result number into columns of the same sheet so that the run script can
// dump the sheet. Nothing here is copied from OriginLab code; it only calls documented functions.
#include <Origin.h>
#include <stats_utils.h>

static void put(Worksheet &wks, int col, const vector<double> &v)
{
	while (wks.GetNumCols() <= col)
		wks.AddCol();
	Dataset ds(wks, col);
	ds.SetSize(v.GetSize());
	for (int ip = 0; ip < v.GetSize(); ip++)
		ds[ip] = v[ip];
}

// One-way ANOVA of the columns listed in vcols (0-based) with the means comparison of the given type.
// Results go to column 'at' (the ANOVA numbers) and the nine columns after it (the comparisons).
double sheaf_anova(int c1, int c2, int c3, int method, int at, double alpha)
{
	Worksheet wks = Project.ActiveLayer();
	int cols[3];
	cols[0] = c1;
	cols[1] = c2;
	cols[2] = c3;
	vector<double> data;
	int k = 3;
	vector<int> gs(k);
	for (int ga = 0; ga < k; ga++)
	{
		Dataset ds(wks, cols[ga]);
		for (int ib = 0; ib < ds.GetSize(); ib++)
		{
			if (!is_missing_value(ds[ib]))
			{
				data.Add(ds[ib]);
				gs[ga] = gs[ga] + 1;
			}
		}
	}
	ANOVADescStats desc[3];
	ANOVAStats atot;
	RegANOVA atab;
	int rc = ocmath_anova_one_way(data, data.GetSize(), k, gs, desc, k, &atot, &atab);
	vector<double> flat;
	flat.Add(rc);
	for (int gc = 0; gc < k; gc++)
	{
		flat.Add(desc[gc].N);
		flat.Add(desc[gc].Mean);
		flat.Add(desc[gc].SD);
		flat.Add(desc[gc].SEM);
	}
	for (int re = 0; re < 3; re++)
	{
		flat.Add(atab.AnovaTable[re].DOF);
		flat.Add(atab.AnovaTable[re].SSq);
		flat.Add(atab.AnovaTable[re].MeanSq);
	}
	flat.Add(atab.Fvalue);
	flat.Add(atab.Pvalue);
	flat.Add(atot.RSQ);
	flat.Add(atot.COV);
	flat.Add(atot.MSESQ);
	flat.Add(atot.Mean);
	put(wks, at, flat);

	int npair = k * (k - 1) / 2;
	vector<double> means(k);
	vector<int> ns(k);
	for (int gd = 0; gd < k; gd++)
	{
		means[gd] = desc[gd].Mean;
		ns[gd] = desc[gd].N;
	}
	vector<int> ic(npair);
	vector<double> diff(npair), lcl(npair), ucl(npair), se(npair), stat(npair), prob(npair), al(npair);
	vector<long> sig(npair);
	int rc2 = ocmath_anova_mean_comparison(means, k, ns, k, atab.AnovaTable[1].MeanSq, (int)atab.AnovaTable[1].DOF,
		alpha, k, ic, npair, diff, npair, lcl, npair, ucl, npair, se, npair, stat, npair, prob, npair, sig,
		npair, al, npair, method);
	vector<double> v;
	v.Add(rc2);
	put(wks, at + 1, v);
	vector<double> icd(npair), sigd(npair);
	for (int if1 = 0; if1 < npair; if1++)
		icd[if1] = ic[if1];
	for (int if2 = 0; if2 < npair; if2++)
		sigd[if2] = sig[if2];
	put(wks, at + 2, icd);
	put(wks, at + 3, diff);
	put(wks, at + 4, lcl);
	put(wks, at + 5, ucl);
	put(wks, at + 6, se);
	put(wks, at + 7, stat);
	put(wks, at + 8, prob);
	put(wks, at + 9, sigd);
	put(wks, at + 10, al);
	return rc;
}

// The normality tests on column 'c'; the numbers go to column 'at'.
double sheaf_norm(int c, int at)
{
	Worksheet wks = Project.ActiveLayer();
	Dataset ds(wks, c);
	vector<double> data;
	for (int ig = 0; ig < ds.GetSize(); ig++)
		if (!is_missing_value(ds[ig]))
			data.Add(ds[ig]);
	vector<double> flat;
	NormTestResults sw, lf, ks;
	int r1 = stats_shapiro_wilk_test(data, sw, 0);
	flat.Add(r1);
	flat.Add(sw.TestStat);
	flat.Add(sw.DOF);
	flat.Add(sw.Prob);
	int r2 = stats_lilliefors_test(data, lf);
	flat.Add(r2);
	flat.Add(lf.TestStat);
	flat.Add(lf.DOF);
	flat.Add(lf.Prob);
	vector par(2);
	int r3 = stats_kolmogorov_smirnov_test(data, PARA_ESTIMATED, par, ks);
	flat.Add(r3);
	flat.Add(ks.TestStat);
	flat.Add(ks.DOF);
	flat.Add(ks.Prob);
	flat.Add(par[0]);
	flat.Add(par[1]);
	NormTestResults2 ad;
	int r4 = stats_anderson_darling_test(data, ad);
	flat.Add(r4);
	flat.Add(ad.TestStat);
	flat.Add(ad.Prob);
	NormTestResults2 chi, skew, kurt;
	int re = stats_k_squared_test(data, chi, skew, kurt);
	flat.Add(re);
	flat.Add(chi.TestStat);
	flat.Add(chi.Prob);
	flat.Add(skew.TestStat);
	flat.Add(skew.Prob);
	flat.Add(kurt.TestStat);
	flat.Add(kurt.Prob);
	put(wks, at, flat);
	return r1;
}

// Origin's automatic bin range and size (the library function the histogram and frequency count tools use
// when the range and the size are left on automatic), evaluated for the rows of three input columns:
// the number of points, the minimum and the maximum. The bin start, bin end and bin size go to the
// three columns starting at 'at'.
double sheaf_round(int cn, int cmin, int cmax, int at)
{
	Worksheet wks = Project.ActiveLayer();
	Dataset dn(wks, cn);
	Dataset dlo(wks, cmin);
	Dataset dhi(wks, cmax);
	int rows = dn.GetSize();
	vector<double> o1(rows), o2(rows), o3(rows);
	for (int ir = 0; ir < rows; ir++)
	{
		double lo = dlo[ir];
		double hi = dhi[ir];
		double inc = 0;
		RoundBinningRange(&lo, &hi, (int)dn[ir], LINEAR_SPACE, &inc);
		o1[ir] = lo;
		o2[ir] = hi;
		o3[ir] = inc;
	}
	put(wks, at, o1);
	put(wks, at + 1, o2);
	put(wks, at + 2, o3);
	return rows;
}
