// Our own Origin C helper for the two-way ANOVA reference runs (stats-anova2.lines.txt).
// It reads the numeric columns A (factor A level number), B (factor B level number) and C (data) of the
// active sheet, calls Origin's public statistics library functions for the two-way ANOVA, and writes every
// result number into columns of the same sheet so that the run script can dump the sheet. Nothing here is
// copied from OriginLab code; it only calls documented functions.
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

static void putcomp(Worksheet &wks, int at, int test, const vector<int> &fac, int nlev, const vector<double> &dep,
	ANOVA2Table *tab, double alpha)
{
	int npair = nlev * (nlev - 1) / 2;
	MeanCompStats mc[40];
	int rc = stats_anova_two_way_means_comparison(test, fac, nlev, dep, tab, alpha, mc, npair);
	vector<double> c0, c1, c2, c3, c4, c5, c6, c7, c8;
	c0.Add(rc);
	for (int i = 0; i < npair; i++)
	{
		c1.Add(mc[i].IndexComp);
		c2.Add(mc[i].MeanDiff);
		c3.Add(mc[i].SEM);
		c4.Add(mc[i].DOF);
		c5.Add(mc[i].Stat);
		c6.Add(mc[i].Prob);
		c7.Add(mc[i].Sig);
		c8.Add(mc[i].LCL);
	}
	vector<double> c9;
	for (int j = 0; j < npair; j++)
		c9.Add(mc[j].UCL);
	put(wks, at, c0);
	put(wks, at + 1, c1);
	put(wks, at + 2, c2);
	put(wks, at + 3, c3);
	put(wks, at + 4, c4);
	put(wks, at + 5, c5);
	put(wks, at + 6, c6);
	put(wks, at + 7, c7);
	put(wks, at + 8, c8);
	put(wks, at + 9, c9);
}

// inter: 1 with interaction; method: 0 none, else the means comparison type of the library (1 Tukey,
// 2 Bonferroni, 3 Sidak, 4 Fisher, 5 Scheffe; the library's header lists another order, these are measured).
double sheaf_anova2(int inter, int nA, int nB, int method, double alpha)
{
	Worksheet wks = Project.ActiveLayer();
	Dataset da(wks, 0), db(wks, 1), dc(wks, 2);
	vector<int> fa, fb;
	vector<double> dep;
	for (int i = 0; i < dc.GetSize(); i++)
	{
		if (!is_missing_value(dc[i]))
		{
			fa.Add((int)da[i]);
			fb.Add((int)db[i]);
			dep.Add(dc[i]);
		}
	}
	ANOVA2DescStats ad[10], bd[10], td, abd[100];
	ANOVA2Table tab[6];
	int rc = stats_anova_two_way_compute_anova_table(inter, fa, nA, fb, nB, dep, ad, bd, td, abd, tab);
	vector<double> t;
	t.Add(rc);
	for (int r = 0; r < 6; r++)
	{
		t.Add(tab[r].DOF);
		t.Add(tab[r].SSq);
		t.Add(tab[r].MeanSq);
		t.Add(tab[r].FValue);
		t.Add(tab[r].PValue);
	}
	put(wks, 3, t);
	vector<double> va, vb, vab, vt;
	for (int i1 = 0; i1 < nA; i1++)
	{
		va.Add(ad[i1].N);
		va.Add(ad[i1].Mean);
		va.Add(ad[i1].SD);
		va.Add(ad[i1].SEM);
	}
	for (int i2 = 0; i2 < nB; i2++)
	{
		vb.Add(bd[i2].N);
		vb.Add(bd[i2].Mean);
		vb.Add(bd[i2].SD);
		vb.Add(bd[i2].SEM);
	}
	for (int i3 = 0; i3 < nA * nB; i3++)
	{
		vab.Add(abd[i3].N);
		vab.Add(abd[i3].Mean);
		vab.Add(abd[i3].SD);
		vab.Add(abd[i3].SEM);
	}
	vt.Add(td.N);
	vt.Add(td.Mean);
	vt.Add(td.SD);
	vt.Add(td.SEM);
	put(wks, 4, va);
	put(wks, 5, vb);
	put(wks, 6, vab);
	put(wks, 7, vt);
	if (method > 0 && rc == 0)
	{
		putcomp(wks, 8, method, fa, nA, dep, tab, alpha);
		putcomp(wks, 18, method, fb, nB, dep, tab, alpha);
		if (inter)
		{
			vector<int> comb;
			for (int k = 0; k < fa.GetSize(); k++)
				comb.Add((fa[k] - 1) * nB + fb[k]);
			putcomp(wks, 28, method, comb, nA * nB, dep, tab, alpha);
		}
	}
	return rc;
}

// One-way ANOVA means comparison of k (up to 6) groups in the first k columns of the active sheet; the results
// go to columns k.. : the pairs' index, mean difference, standard error, statistic, probability, lcl, ucl.
double sheaf_anova1k(int k, int method, double alpha)
{
	Worksheet wks = Project.ActiveLayer();
	vector<double> data;
	vector<int> gs(k);
	for (int ga = 0; ga < k; ga++)
	{
		Dataset ds(wks, ga);
		for (int ib = 0; ib < ds.GetSize(); ib++)
		{
			if (!is_missing_value(ds[ib]))
			{
				data.Add(ds[ib]);
				gs[ga] = gs[ga] + 1;
			}
		}
	}
	ANOVADescStats desc[6];
	ANOVAStats atot;
	RegANOVA atab;
	int rc = ocmath_anova_one_way(data, data.GetSize(), k, gs, desc, k, &atot, &atab);
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
	vector<double> icd(npair);
	for (int i1 = 0; i1 < npair; i1++)
		icd[i1] = ic[i1];
	put(wks, k, icd);
	put(wks, k + 1, diff);
	put(wks, k + 2, se);
	put(wks, k + 3, stat);
	put(wks, k + 4, prob);
	put(wks, k + 5, lcl);
	put(wks, k + 6, ucl);
	return rc + rc2;
}
